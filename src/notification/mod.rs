mod web_push;
mod smtp_client;
mod smtp_server;

use crate::config::{AppConfig, MailSendType};
use actix_web::{Scope, web};

pub use web_push::{get_vapid_public_key, web_push_subscribe, associate_guest, send_push};
pub use smtp_client::send_emails_via_client;
pub use smtp_server::send_emails_direct;

/// Main email sending function that chooses between client and direct sending
pub async fn send_emails(
    db: web::Data<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>,
    config: &AppConfig,
    subject: String,
    body: String,
    guest_ids: Vec<String>,
) -> Result<(), String> {
    match &config.mail_sendtype {
        MailSendType::Client => send_emails_via_client(db, config, subject, body, guest_ids).await,
        MailSendType::Direct => send_emails_direct(db, config, subject, body, guest_ids).await,
        MailSendType::Auto | MailSendType::Invalid(_) => {
            if config.smtp_client_configured() {
                send_emails_via_client(db, config, subject, body, guest_ids).await
            } else {
                send_emails_direct(db, config, subject, body, guest_ids).await
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
