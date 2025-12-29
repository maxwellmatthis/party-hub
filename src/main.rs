use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use db::prepare_db;
use r2d2_sqlite::SqliteConnectionManager;
use std::env;
use std::fs;
mod auth;
mod db;
mod guest;
mod invitation;
mod party;

#[get("/static/{filename}")]
async fn serve_static(path: web::Path<String>) -> impl Responder {
    let filename = path.into_inner();

    // Whitelist of allowed static files for security
    let allowed_files = vec![
        "auth.css",
        "calendar.svg",
        "chevrons-down.svg",
        "chevrons-up.svg",
        "chevron-down.svg",
        "chevron-up.svg",
        "clipboard.svg",
        "index.css",
        "index.js",
        "invitation.css",
        "invitation.js",
        "manage.css",
        "manage.js",
        "menu.svg",
        "plus.svg",
        "style.css",
        "trash-2.svg",
        "whyemptycat.png",
        "favicon.ico",
        "x.svg",
        "public_guest.js",
    ];

    // Check if the requested file is in the whitelist
    if !allowed_files.contains(&filename.as_str()) {
        return HttpResponse::NotFound().body("File not found");
    }

    // Construct the file path
    let file_path = format!("static/{}", filename);

    // Read the file
    let file_content = match fs::read(&file_path) {
        Ok(content) => content,
        Err(_) => return HttpResponse::NotFound().body("File not found"),
    };

    // Determine content type based on file extension
    let content_type = match filename.split('.').last() {
        Some("js") => "application/javascript",
        Some("css") => "text/css",
        Some("html") => "text/html",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("gif") => "image/gif",
        Some("ico") => "image/x-icon",
        Some("json") => "application/json",
        Some("txt") => "text/plain",
        _ => "application/octet-stream",
    };

    HttpResponse::Ok()
        .content_type(content_type)
        .body(file_content)
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
    prepare_db().expect("FATAL: Unable to set up DB!");
    let manager = SqliteConnectionManager::file("party.db");
    let pool = r2d2::Pool::new(manager).unwrap();

    // Get port from environment variable, default to 8080
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    println!("INFO: Starting Party Hub server on http://127.0.0.1:{}", port);
    match env::var("ENV") {
        Ok(val) => {
            println!("INFO: Running in {val} mode.")
        },
        Err(_e) => println!("WARNING: No ENV found in environment. Defaulting to production mode. FOR SECURITY REASONS COOKIES WILL NO WORK WITHOUT HTTPS.")
    }

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(serve_static)
            .service(auth::subroutes())
            .service(guest::subroutes())
            .service(party::subroutes())
            .service(party::home)
            .service(party::dashboard)
            .service(invitation::subroutes())
            .service(invitation::register)
            .service(invitation::invitation_page)
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
