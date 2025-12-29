use actix_web::web;
use lettre::{Message, SmtpTransport, Transport};
use lettre::transport::smtp::client::{Tls, TlsParameters};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use hickory_resolver::TokioAsyncResolver;
use std::sync::OnceLock;
use uuid::Uuid;

static SMTP_DIRECT_CONFIGURED: OnceLock<bool> = OnceLock::new();

pub fn is_smtp_direct_configured() -> bool {
    *SMTP_DIRECT_CONFIGURED.get_or_init(|| {
        std::env::var("SMTP_FROM").is_ok()
    })
}

/// Get MX records for a domain, sorted by priority (lower number = higher priority)
async fn get_mx_records(domain: &str) -> Result<Vec<(u16, String)>, String> {
    let resolver = TokioAsyncResolver::tokio_from_system_conf()
        .map_err(|e| format!("Failed to create DNS resolver: {}", e))?;

    let mx_records = resolver.mx_lookup(domain).await
        .map_err(|e| format!("Failed to resolve MX records for {}: {}", domain, e))?;

    let mut records: Vec<(u16, String)> = mx_records
        .iter()
        .map(|mx| (mx.preference(), mx.exchange().to_utf8()))
        .collect();

    // Sort by priority (lower number = higher priority)
    records.sort_by_key(|r| r.0);

    Ok(records)
}

/// Send email directly to recipient's mail server (bypasses sender's SMTP server)
/// This is useful when you have SPF/DKIM configured for your domain
async fn send_email_direct(
    from_addr: &str,
    to_addr: &str,
    to_email: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    // Extract domain from recipient email
    let to_domain = to_email.split('@').nth(1)
        .ok_or_else(|| "Invalid recipient email address".to_string())?;

    // Get MX records for the recipient's domain
    let mx_records = get_mx_records(to_domain).await?;

    if mx_records.is_empty() {
        return Err(format!("No MX records found for {}", to_domain));
    }

    // Generate a unique Message-ID
    let from_domain = from_addr.split('@').nth(1)
        .and_then(|s| s.split('>').next())
        .unwrap_or("localhost");
    let message_id = format!("<{}.{}@{}>", 
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        Uuid::new_v4(),
        from_domain
    );

    // Create the email message with Message-ID header
    let message = Message::builder()
        .from(from_addr.parse().map_err(|e| format!("Invalid from address: {}", e))?)
        .to(to_addr.parse().map_err(|e| format!("Invalid to address: {}", e))?)
        .subject(subject)
        .message_id(Some(message_id))
        .body(body.to_string())
        .map_err(|e| format!("Failed to build email: {}", e))?;

    // Try each MX server in order of priority
    for (priority, mx_host) in mx_records {
        // For direct MX connections, we need to be more lenient with TLS
        // Many MX servers have certificates that don't exactly match their hostname
        let tls = match TlsParameters::builder(mx_host.clone())
            .dangerous_accept_invalid_hostnames(true)
            .build()
        {
            Ok(params) => params,
            Err(e) => {
                eprintln!("[EMAIL] Failed to create TLS parameters for {}: {}", mx_host, e);
                continue;
            }
        };

        // Try to connect and send
        let mailer = match SmtpTransport::relay(&mx_host) {
            Ok(builder) => builder
                .tls(Tls::Opportunistic(tls))
                .port(25)
                .build(),
            Err(e) => {
                eprintln!("[EMAIL] Failed to create SMTP transport for {}: {}", mx_host, e);
                continue;
            }
        };

        match mailer.send(&message) {
            Ok(_) => {
                return Ok(());
            }
            Err(e) => {
                eprintln!("[EMAIL] Failed to send via {}: {}", mx_host, e);
                continue;
            }
        }
    }

    Err(format!("Failed to send email via any MX server for {}", to_domain))
}

/// Sends emails to guests directly to their mail servers (requires SPF/DKIM setup)
/// This bypasses your SMTP provider and sends directly to recipient's mail servers
/// Requires environment variable: SMTP_FROM (sender address)
pub async fn send_emails_direct(
    db: web::Data<Pool<SqliteConnectionManager>>,
    subject: String,
    body: String,
    guest_ids: Vec<String>,
) -> Result<(), String> {
    // Get from address from environment
    let smtp_from = match std::env::var("SMTP_FROM") {
        Ok(addr) => addr,
        Err(_) => {
            return Ok(());
        }
    };

    let conn = db.get()
        .map_err(|_| "Database connection failed")?;

    // Get email addresses for all guests
    let placeholders = guest_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let query = format!(
        "SELECT email, first, last FROM guests WHERE id IN ({}) AND email != ''",
        placeholders
    );

    let mut stmt = conn.prepare(&query)
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

    // Send email to each guest
    for (email, first, last) in guest_emails {
        let recipient_name = format!("{} {}", first, last).trim().to_string();
        let to_addr = format!("{} <{}>", recipient_name, email);

        match send_email_direct(&smtp_from, &to_addr, &email, &subject, &body).await {
            Ok(_) => {
            }
            Err(e) => {
                eprintln!("[EMAIL ERROR] Failed to send to {}: {}", email, e);
            }
        }
    }

    Ok(())
}
