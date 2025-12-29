/**
 * Converts a base64-encoded VAPID key to Uint8Array format
 */
function urlBase64ToUint8Array(base64String) {
    const padding = '='.repeat((4 - (base64String.length % 4)) % 4);
    const base64 = (base64String + padding)
        .replace(/-/g, '+')
        .replace(/_/g, '/');

    const rawData = window.atob(base64);
    const outputArray = new Uint8Array(rawData.length);

    for (let i = 0; i < rawData.length; ++i) {
        outputArray[i] = rawData.charCodeAt(i);
    }
    return outputArray;
}

let serviceWorkerSupported = false;
let pushSupported = false;

function checkSupport() {
    serviceWorkerSupported = 'serviceWorker' in navigator;
    pushSupported = 'PushManager' in window;

    return serviceWorkerSupported && pushSupported;
}

/** Check whether the user already has a service worker installed. */
async function checkWorkerAlreadyInstalled() {
    checkSupport();
    if (serviceWorkerSupported) {
        try {
            const registrations = await navigator.serviceWorker.getRegistrations();
            if (registrations.length > 0) {
                return true;
            }
        } catch (error) {
            console.error('Error checking service worker registrations:', error);
        }
    }
    return false;
}

/**
 * Registers the service worker for push notifications
 */
async function registerServiceWorker() {
    if (!serviceWorkerSupported) {
        console.error("Service Worker isn't supported on this browser.");
        throw 'Service Worker not supported';
    }

    if (await checkWorkerAlreadyInstalled()) {
        return true;
    }

    try {
        await navigator.serviceWorker.register('/web-push-service-worker.js');
        return true;
    } catch (error) {
        console.error("Failed to register service worker:", error);
        return false;
    }
}

/**
 * Requests notification permission from the user
 */
async function askPermission() {
    if (!pushSupported) {
        console.error("Push isn't supported on this browser.");
        throw 'Push not supported';
    }

    const permissionStatus = await new Promise((resolve, reject) => {
        const permissionResult = Notification.requestPermission(
            (result) => { resolve(result); },
        );

        if (permissionResult) {
            permissionResult.then(resolve, reject);
        }
    });

    if (permissionStatus === 'granted') {
        return true;
    }
    return false;
}

/**
 * Subscribes the user to push notifications using VAPID keys
 */
function subscribeUserToPush() {
    return navigator.serviceWorker
        .register('/web-push-service-worker.js')
        .then(async function (registration) {
            const response = await fetch('/notification/vapid-public-key');
            const data = await response.json();
            
            if (!data?.publicKey) {
                throw new Error('Failed to get VAPID public key');
            }

            const subscribeOptions = {
                userVisibleOnly: true,
                applicationServerKey: urlBase64ToUint8Array(data.publicKey),
            };

            return registration.pushManager.subscribe(subscribeOptions);
        })
        .then(function (pushSubscription) {
            return pushSubscription;
        });
}

/**
 * Complete subscription flow: subscribe and register with server
 * @param {string} guestId - The guest ID to associate with this subscription
 */
export async function subscribe(guestId) {
    if (!(checkSupport() && await registerServiceWorker() && await askPermission())) {
        return false;
    }

    const subscriptionObject = await subscribeUserToPush();
    const subscription = subscriptionObject.toJSON();

    const data = {
        endpoint: subscription.endpoint || "",
        p256dh: subscription.keys?.p256dh || "",
        auth: subscription.keys?.auth || ""
    };

    const response = await fetch(`/notification/web-push-subscribe/${guestId}`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(data)
    });

    if (response.ok) {
        // Store the endpoint as a device identifier in localStorage
        // We'll use this to associate other guests with this device
        localStorage.setItem('party_hub_device_endpoint', subscription.endpoint);
    }

    return response.ok;
}

/**
 * Associate a guest with the existing device subscription
 * @param {string} guestId - The guest ID to associate
 */
export async function associateGuestWithDevice(guestId) {
    const endpoint = localStorage.getItem('party_hub_device_endpoint');
    
    if (!endpoint) {
        // No device subscription exists, try to create one
        return await subscribe(guestId);
    }

    // Try to associate this guest with the existing subscription
    const response = await fetch(`/notification/associate-guest/${guestId}`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({ endpoint: endpoint })
    });

    return response.ok;
}
