mod web_push;
mod smtp_client;
mod smtp_server;

use actix_web::{Scope, web};

pub use web_push::{get_vapid_public_key, web_push_subscribe, associate_guest, send_push};
pub use smtp_client::{send_emails_via_client, is_smtp_client_configured};
pub use smtp_server::{send_emails_direct, is_smtp_direct_configured};

/// Main email sending function that chooses between client and direct sending
/// Respects MAIL_SENDTYPE environment variable ("client" or "direct")
pub async fn send_emails(
    db: web::Data<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>,
    subject: String,
    body: String,
    guest_ids: Vec<String>,
) -> Result<(), String> {
    // Check MAIL_SENDTYPE preference
    match std::env::var("MAIL_SENDTYPE").as_deref() {
        Ok("client") => {
            send_emails_via_client(db, subject, body, guest_ids).await
        }
        Ok("direct") => {
            send_emails_direct(db, subject, body, guest_ids).await
        }
        _ => {
            // No preference set, try client first, fall back to direct
            if is_smtp_client_configured() {
                send_emails_via_client(db, subject, body, guest_ids).await
            } else {
                send_emails_direct(db, subject, body, guest_ids).await
            }
        }
    }
}

pub fn subroutes() -> Scope {
    web::scope("/notification")
        .service(get_vapid_public_key)
        .service(web_push_subscribe)
        .service(associate_guest)
}
