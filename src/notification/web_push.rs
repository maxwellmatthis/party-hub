use actix_web::{HttpResponse, Responder, get, post, web};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use web_push::{
    ContentEncoding, IsahcWebPushClient, SubscriptionInfo, VapidSignatureBuilder, WebPushClient,
    WebPushMessageBuilder,
};

/// Web push subscription parameters from the client
#[derive(Deserialize)]
pub struct WebPushSubscriptionOptions {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
}

/// Associate guest with existing subscription
#[derive(Deserialize)]
pub struct AssociateGuestRequest {
    pub endpoint: String,
}

/// Returns the VAPID public key for web push subscriptions
#[get("/vapid-public-key")]
pub async fn get_vapid_public_key() -> impl Responder {
    match std::fs::read_to_string("public_vapid_key.pem") {
        Ok(key) => HttpResponse::Ok().json(json!({
            "publicKey": key.trim()
        })),
        Err(e) => {
            eprintln!("[FILE ERROR] Failed to read VAPID public key: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Stores a web push subscription for a user
#[post("/web-push-subscribe/{guest_id}")]
pub async fn web_push_subscribe(
    path: web::Path<String>,
    form: web::Json<WebPushSubscriptionOptions>,
    db: web::Data<Pool<SqliteConnectionManager>>,
) -> impl Responder {
    let guest_id = path.into_inner();

    let conn = match db.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("Database connection failed"),
    };

    let sub_id = Uuid::new_v4().to_string();

    // Insert or update the subscription (use endpoint as unique key)
    let result = conn
        .prepare(
            "INSERT INTO web_push_subscriptions (id, endpoint, p256dh, auth) 
             VALUES (?1, ?2, ?3, ?4) 
             ON CONFLICT(endpoint) DO UPDATE SET p256dh = ?3, auth = ?4",
        )
        .and_then(|mut stmt| {
            stmt.execute([&sub_id, &form.endpoint, &form.p256dh, &form.auth])
        });

    if let Err(e) = result {
        eprintln!(
            "[DATABASE ERROR] Failed to insert web push subscription: {}",
            e
        );
        return HttpResponse::InternalServerError().finish();
    }

    // Get the actual subscription_id (either the new one or existing one)
    let actual_sub_id: String = match conn
        .prepare("SELECT id FROM web_push_subscriptions WHERE endpoint = ?1")
        .and_then(|mut stmt| stmt.query_row([&form.endpoint], |row| row.get(0)))
    {
        Ok(id) => id,
        Err(e) => {
            eprintln!("[DATABASE ERROR] Failed to retrieve subscription id: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    // Link guest to subscription
    let link_result = conn
        .prepare(
            "INSERT INTO guest_subscriptions (guest_id, subscription_id) 
             VALUES (?1, ?2) 
             ON CONFLICT(guest_id, subscription_id) DO NOTHING",
        )
        .and_then(|mut stmt| stmt.execute([&guest_id, &actual_sub_id]));

    match link_result {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            eprintln!(
                "[DATABASE ERROR] Failed to link guest to subscription: {}",
                e
            );
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Associates a guest with an existing device subscription
#[post("/associate-guest/{guest_id}")]
pub async fn associate_guest(
    path: web::Path<String>,
    form: web::Json<AssociateGuestRequest>,
    db: web::Data<Pool<SqliteConnectionManager>>,
) -> impl Responder {
    let guest_id = path.into_inner();

    let conn = match db.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("Database connection failed"),
    };

    // Find the subscription_id by endpoint
    let subscription_id: String = match conn
        .prepare("SELECT id FROM web_push_subscriptions WHERE endpoint = ?1")
        .and_then(|mut stmt| stmt.query_row([&form.endpoint], |row| row.get(0)))
    {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::NotFound().body("Subscription not found for this device");
        }
    };

    // Link guest to subscription
    let link_result = conn
        .prepare(
            "INSERT INTO guest_subscriptions (guest_id, subscription_id) 
             VALUES (?1, ?2) 
             ON CONFLICT(guest_id, subscription_id) DO NOTHING",
        )
        .and_then(|mut stmt| stmt.execute([&guest_id, &subscription_id]));

    match link_result {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            eprintln!(
                "[DATABASE ERROR] Failed to link guest to subscription: {}",
                e
            );
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Sends push notifications to guests with invitation links
/// A single device (subscription) can receive notifications for multiple guests
pub async fn send_push(
    db: web::Data<Pool<SqliteConnectionManager>>,
    content: String,
    guest_invitation_map: std::collections::HashMap<String, String>,
) -> impl Responder {
    let conn = match db.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let private_key = match std::fs::read_to_string("private_vapid_key.pem") {
        Ok(key) => key.trim().to_string(),
        Err(e) => {
            eprintln!("[FILE ERROR] Failed to read VAPID key: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    // Build query with placeholders for all guest_ids
    let guest_ids: Vec<String> = guest_invitation_map.keys().cloned().collect();
    let placeholders = guest_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let query = format!(
        "SELECT DISTINCT gs.guest_id, ws.endpoint, ws.p256dh, ws.auth 
         FROM guest_subscriptions gs 
         JOIN web_push_subscriptions ws ON gs.subscription_id = ws.id 
         WHERE gs.guest_id IN ({})",
        placeholders
    );

    let mut stmt = match conn.prepare(&query) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("[DATABASE ERROR] Failed to prepare query: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    // Convert guest_ids to rusqlite::params
    let params: Vec<&dyn rusqlite::ToSql> = guest_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect();

    let subscription_iter = match stmt.query_map(params.as_slice(), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    }) {
        Ok(iter) => iter,
        Err(e) => {
            eprintln!("[DATABASE ERROR] Failed to execute query: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let client = match IsahcWebPushClient::new() {
        Ok(client) => client,
        Err(e) => {
            eprintln!("[WEB PUSH ERROR] Failed to create client: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    for subscription_result in subscription_iter {
        let (guest_id, endpoint, p256dh, auth) = match subscription_result {
            Ok(data) => data,
            Err(e) => {
                eprintln!("[DATABASE ERROR] Failed to read subscription: {}", e);
                continue;
            }
        };

        // Get the invitation_id for this guest
        let invitation_id = match guest_invitation_map.get(&guest_id) {
            Some(id) => id,
            None => continue,
        };

        let subscription_info = SubscriptionInfo::new(&endpoint, &p256dh, &auth);

        let vapid_signature =
            match VapidSignatureBuilder::from_base64(&private_key, &subscription_info) {
                Ok(builder) => match builder.build() {
                    Ok(sig) => sig,
                    Err(e) => {
                        eprintln!("[VAPID ERROR] Failed to build signature: {}", e);
                        continue;
                    }
                },
                Err(e) => {
                    eprintln!("[VAPID ERROR] Failed to create signature builder: {}", e);
                    continue;
                }
            };

        // Create JSON payload with message and invitation URL
        let payload = json!({
            "message": content,
            "url": format!("/{}", invitation_id)
        });
        let payload_str = payload.to_string();

        let mut builder = WebPushMessageBuilder::new(&subscription_info);
        builder.set_payload(ContentEncoding::Aes128Gcm, payload_str.as_bytes());
        builder.set_vapid_signature(vapid_signature);

        let message = match builder.build() {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("[WEB PUSH ERROR] Failed to build message: {}", e);
                continue;
            }
        };

       match client.send(message).await {
            Ok(_) => {}
            Err(_e) => {}
        }
    }

    HttpResponse::Ok().finish()
}
