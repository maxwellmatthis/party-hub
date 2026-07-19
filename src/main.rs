use actix_files::NamedFile;
use actix_web::{App, HttpResponse, HttpServer, get, web};
mod config;
use db::prepare_db;
use r2d2_sqlite::SqliteConnectionManager;
use crate::config::{AppConfig, MailSendType};
use std::fs;
mod auth;
mod db;
mod guest;
mod invitation;
mod notification;
mod party;

#[get("/static/{filename:.*}")]
async fn serve_static(
    path: web::Path<String>,
    config: web::Data<AppConfig>,
) -> actix_web::Result<NamedFile> {
    let filename = path.into_inner();
    let file_path = format!("{}/static/{}", config.web_assets_path, filename);
    Ok(NamedFile::open(file_path)?)
}

#[get("/web-push-service-worker.js")]
async fn serve_service_worker(config: web::Data<AppConfig>) -> HttpResponse {
    let file_path = format!("{}/static/web-push-service-worker.js", config.web_assets_path);

    let file_content = match fs::read(file_path) {
        Ok(content) => content,
        Err(_) => return HttpResponse::NotFound().body("Service worker script not found"),
    };

    HttpResponse::Ok()
        .content_type("application/javascript")
        .insert_header(("Service-Worker-Allowed", "/"))
        .body(file_content)
}

#[get("/favicon.ico")]
async fn serve_favicon(config: web::Data<AppConfig>) -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open(format!("{}/static/logo/favicon.ico", config.web_assets_path))?)
}

#[get("/manifest.json")]
async fn serve_manifest(config: web::Data<AppConfig>) -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open(format!("{}/static/manifest.json", config.web_assets_path))?)
}

fn detect_language(req: &actix_web::HttpRequest) -> String {
    if let Some(accept_lang) = req.headers().get("accept-language") {
        if let Ok(lang_str) = accept_lang.to_str() {
            // Parse Accept-Language header (e.g., "de-DE,de;q=0.8,en;q=0.6")
            for lang_part in lang_str.split(',') {
                let lang_code = lang_part.split(';').next().unwrap_or("").trim();

                // Check for German variants
                if lang_code.starts_with("de") {
                    return "de".to_string();
                }
                // Default to English for any other language
            }
        }
    }
    "en".to_string() // Default to English
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = AppConfig::from_env();
    prepare_db().expect("FATAL: Unable to set up DB!");
    let manager = SqliteConnectionManager::file(&config.sqlite_db_path);
    let pool = r2d2::Pool::new(manager).unwrap();

    println!(
        "INFO: Starting Party Hub server on http://127.0.0.1:{}",
        config.port
    );
    match config.env_mode.as_deref() {
        Some(val) => println!("INFO: Running in {val} mode."),
        None => println!(
            "WARNING: No ENV found in environment. Defaulting to production mode. FOR SECURITY REASONS COOKIES WILL NOT WORK WITHOUT HTTPS."
        ),
    }

    // Check email configuration and warn if not set up properly
    let smtp_client_configured = config.smtp_client_configured();
    let smtp_direct_configured = config.smtp_direct_configured();

    match &config.mail_sendtype {
        MailSendType::Client => {
            if smtp_client_configured {
                println!(
                    "INFO: Email notifications enabled via SMTP client (MAIL_SENDTYPE=client)"
                );
            } else {
                println!("WARNING: MAIL_SENDTYPE set to 'client' but SMTP client not configured.");
                println!(
                    "         Set SMTP_SERVER, SMTP_USERNAME, SMTP_PASSWORD, and SMTP_FROM to enable."
                );
            }
        }
        MailSendType::Direct => {
            if smtp_direct_configured {
                println!(
                    "INFO: Email notifications enabled via direct SMTP (MAIL_SENDTYPE=direct)"
                );
                println!(
                    "      Make sure your domain has proper SPF/DKIM/DMARC records configured."
                );
            } else {
                println!("WARNING: MAIL_SENDTYPE set to 'direct' but direct SMTP not configured.");
                println!("         Set SMTP_FROM to enable direct SMTP sending.");
            }
        }
        MailSendType::Invalid(other) => {
            println!(
                "WARNING: Invalid MAIL_SENDTYPE value '{}'. Use 'client' or 'direct'.",
                other
            );
            if !smtp_client_configured && !smtp_direct_configured {
                println!("         Email notifications disabled (no email method configured).");
            }
        }
        MailSendType::Auto => {
            // No MAIL_SENDTYPE set, auto-detect
            if smtp_client_configured {
                println!("INFO: Email notifications enabled via SMTP client (auto-detected)");
            } else if smtp_direct_configured {
                println!("INFO: Email notifications enabled via direct SMTP (auto-detected)");
                println!(
                    "      Make sure your domain has proper SPF/DKIM/DMARC records configured."
                );
            } else {
                println!("WARNING: Email notifications disabled (no email method configured).");
                println!(
                    "         Set SMTP_SERVER, SMTP_USERNAME, SMTP_PASSWORD, and SMTP_FROM for SMTP client,"
                );
                println!(
                    "         or set SMTP_FROM for direct SMTP, or use MAIL_SENDTYPE to specify a method."
                );
            }
        }
    }

    let server_config = config.clone();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(server_config.clone()))
            .service(serve_service_worker)
            .service(serve_manifest)
            .service(serve_favicon)
            .service(serve_static)
            .service(auth::subroutes())
            .service(guest::subroutes())
            .service(notification::subroutes())
            .service(party::subroutes())
            .service(party::home)
            .service(party::dashboard)
            .service(invitation::subroutes())
            .service(invitation::register)
            .service(invitation::invitation_page)
    })
    .bind(("127.0.0.1", config.port))?
    .run()
    .await
}
