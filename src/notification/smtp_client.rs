use actix_web::web;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::OnceLock;

static SMTP_CLIENT_CONFIGURED: OnceLock<bool> = OnceLock::new();

pub fn is_smtp_client_configured() -> bool {
    *SMTP_CLIENT_CONFIGURED.get_or_init(|| {
        std::env::var("SMTP_SERVER").is_ok()
            && std::env::var("SMTP_USERNAME").is_ok()
            && std::env::var("SMTP_PASSWORD").is_ok()
            && std::env::var("SMTP_FROM").is_ok()
    })
}

/// Sends emails to guests via SMTP client (authenticated with mail provider)
/// Requires environment variables: SMTP_SERVER, SMTP_USERNAME, SMTP_PASSWORD, SMTP_FROM
/// If not configured, silently skips sending emails
pub async fn send_emails_via_client(
    db: web::Data<Pool<SqliteConnectionManager>>,
    subject: String,
    body: String,
    guest_ids: Vec<String>,
) -> Result<(), String> {
    // Check if SMTP is configured - if not, skip silently
    if !is_smtp_client_configured() {
        return Ok(());
    }

    // Get SMTP configuration from environment
    let smtp_server = std::env::var("SMTP_SERVER").unwrap();
    let smtp_username = std::env::var("SMTP_USERNAME").unwrap();
    let smtp_password = std::env::var("SMTP_PASSWORD").unwrap();
    let smtp_from = std::env::var("SMTP_FROM").unwrap();

    let conn = db.get().map_err(|_| "Database connection failed")?;

    // Get email addresses for all guests
    let placeholders = guest_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let query = format!(
        "SELECT email, first, last FROM guests WHERE id IN ({}) AND email != ''",
        placeholders
    );

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let params: Vec<&dyn rusqlite::ToSql> = guest_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect();

    let guest_emails: Vec<(String, String, String)> = stmt
        .query_map(params.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| format!("Failed to execute query: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    if guest_emails.is_empty() {
        return Ok(()); // No guests with email addresses
    }

    // Create SMTP transport
    let creds = Credentials::new(smtp_username, smtp_password);
    let mailer = SmtpTransport::relay(&smtp_server)
        .map_err(|e| format!("Failed to create SMTP transport: {}", e))?
        .credentials(creds)
        .build();

    // Send email to each guest
    for (email, first, last) in guest_emails {
        let recipient_name = format!("{} {}", first, last).trim().to_string();

        let message = Message::builder()
            .from(
                smtp_from
                    .parse()
                    .map_err(|e| format!("Invalid from address: {}", e))?,
            )
            .to(format!("{} <{}>", recipient_name, email)
                .parse()
                .map_err(|e| format!("Invalid to address: {}", e))?)
            .subject(&subject)
            .body(body.clone())
            .map_err(|e| format!("Failed to build email: {}", e))?;

        match mailer.send(&message) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("[EMAIL ERROR] Failed to send email to {}: {}", email, e);
            }
        }
    }

    Ok(())
}
