use crate::db::Author;
use crate::detect_language;
use actix_web::{
    HttpResponse, Responder, Scope,
    cookie::{Cookie, SameSite},
    get, post, web,
};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::Deserialize;
use std::env;
use std::fs;
use time::{Duration, OffsetDateTime};

#[get("")]
async fn auth(req: actix_web::HttpRequest) -> impl Responder {
    // Detect language from Accept-Language header
    let language = detect_language(&req);

    let filename = match language.as_str() {
        "de" => "pages/de/auth_de.html",
        _ => "pages/en/auth_en.html", // Default to English
    };

    let html_content =
        fs::read_to_string(filename).unwrap_or_else(|_| "<h1>404: File Not Found</h1>".to_string());
    HttpResponse::Ok()
        .content_type("text/html")
        .body(html_content)
}

#[derive(Deserialize)]
struct AuthForm {
    #[serde(rename = "author-secret")]
    author_secret: String,
}

#[post("")]
async fn auth_post(
    form: web::Form<AuthForm>,
    db: web::Data<Pool<SqliteConnectionManager>>,
) -> impl Responder {
    let conn = match db.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("Database connection failed"),
    };

    // Check if the author secret exists in the database
    let author_result = conn
        .prepare("SELECT id, name, author_secret FROM authors WHERE author_secret = ?1")
        .and_then(|mut stmt| stmt.query_row([&form.author_secret], Author::from_row));

    match author_result {
        Ok(_author) => {
            // Create a secure, long-lived cookie with the author secret
            let expiry = OffsetDateTime::now_utc() + Duration::days(90); // 3 months

            // Only use secure=false in development environment
            let is_dev = env::var("ENV").unwrap_or_else(|_| "prod".to_string()) == "dev";

            let cookie = Cookie::build("auth_token", &form.author_secret)
                .path("/")
                .expires(expiry)
                .same_site(SameSite::Lax)
                .http_only(true)
                .secure(!is_dev) // Secure in production, not secure in dev
                .finish();

            HttpResponse::Found()
                .append_header(("Location", "/"))
                .cookie(cookie)
                .finish()
        }
        Err(_) => {
            // Invalid credentials - redirect back to auth page with error
            HttpResponse::Found()
                .append_header(("Location", "/auth?error=invalid"))
                .finish()
        }
    }
}

pub fn is_authenticated_as_author(
    req: &actix_web::HttpRequest,
    db: &Pool<SqliteConnectionManager>,
) -> Option<String> {
    if let Some(cookie) = req.cookie("auth_token") {
        let auth_secret = cookie.value();

        // Validate the token against the database
        if let Ok(conn) = db.get() {
            let author_result = conn
                .prepare("SELECT id, name, author_secret FROM authors WHERE author_secret = ?1")
                .and_then(|mut stmt| stmt.query_row([auth_secret], Author::from_row));

            match author_result {
                Ok(author) => Some(author.id),
                Err(_) => None,
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub fn subroutes() -> Scope {
    web::scope("/auth").service(auth).service(auth_post)
}
