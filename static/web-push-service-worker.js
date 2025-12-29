/**
 * Web Push Service Worker for Party Hub
 * Handles push notifications and VAPID key management
 */

let VAPID_PUBLIC_KEY = null;

/**
 * Fetches and caches the VAPID public key from the server
 * @returns {Promise<string>} The VAPID public key
 */
async function getVapidPublicKey() {
  if (VAPID_PUBLIC_KEY) {
    return VAPID_PUBLIC_KEY;
  }

  try {
    const response = await fetch('/notification/vapid-public-key');
    const data = await response.json();
    VAPID_PUBLIC_KEY = data.publicKey || '';
    return VAPID_PUBLIC_KEY;
  } catch (error) {
    console.error('Failed to fetch VAPID public key:', error);
    return '';
  }
}

/**
 * Converts base64 VAPID key to Uint8Array format
 * @param {string} base64String - The base64 encoded key
 * @returns {Uint8Array} The key as Uint8Array
 */
function urlBase64ToUint8Array(base64String) {
  const padding = '='.repeat((4 - (base64String.length % 4)) % 4);
  const base64 = (base64String + padding)
    .replace(/-/g, '+')
    .replace(/_/g, '/');

  const rawData = atob(base64);
  const outputArray = new Uint8Array(rawData.length);

  for (let i = 0; i < rawData.length; ++i) {
    outputArray[i] = rawData.charCodeAt(i);
  }
  return outputArray;
}

self.addEventListener('push', function(event) {
  console.log('Received a push message', event);

  let title = 'Party Hub';
  let body = 'You have new updates!';
  let icon = '/favicon.ico';
  let url = '/';
  
  if (event.data) {
    try {
      const data = event.data.json();
      body = data.message || body;
      url = data.url || url;
    } catch (e) {
      // If JSON parse fails, try as text
      try {
        body = event.data.text() || body;
      } catch (textError) {
        console.log('No data in push event');
      }
    }
  }

  const options = {
    body: body,
    icon: icon,
    tag: 'party-hub-notification',
    requireInteraction: false,
    data: {
      url: url
    },
    actions: [
      {
        action: 'view',
        title: 'View'
      },
      {
        action: 'close',
        title: 'Close'
      }
    ]
  };

  event.waitUntil(
    self.registration.showNotification(title, options)
  );
});

self.addEventListener('notificationclick', function(event) {
  event.notification.close();

  // Get the URL from notification data, default to home page
  const urlToOpen = event.notification.data?.url || '/';

  // Don't open anything if user clicked 'close'
  if (event.action === 'close') {
    return;
  }

  // Open the URL (either from clicking the notification body or the 'view' action)
  event.waitUntil(
    clients.matchAll({ type: 'window', includeUncontrolled: true }).then(function(clientList) {
      // Check if there's already a window open with this URL
      for (let i = 0; i < clientList.length; i++) {
        const client = clientList[i];
        if (client.url === urlToOpen && 'focus' in client) {
          return client.focus();
        }
      }
      // If no window is open, open a new one
      if (clients.openWindow) {
        return clients.openWindow(urlToOpen);
      }
    })
  );
});

/**
 * Resubscribes to push notifications with current VAPID key
 * @returns {Promise} Promise that resolves when resubscription is complete
 */
function resubscribeToPush() {
  return getVapidPublicKey()
    .then(function(vapidKey) {
      if (!vapidKey) {
        throw new Error('Failed to get VAPID public key');
      }

      return self.registration.pushManager.getSubscription()
        .then(function(subscription) {
          if (subscription) {
            return subscription.unsubscribe();
          }
        })
        .then(function() {
          return self.registration.pushManager.subscribe({
            userVisibleOnly: true,
            applicationServerKey: urlBase64ToUint8Array(vapidKey)
          });
        });
    })
    .then(function(subscription) {
      console.log('Resubscribed to push notifications:', subscription);
    })
    .catch(function(error) {
      console.error('Failed to resubscribe:', error);
    });
}