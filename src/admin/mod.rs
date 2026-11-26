use crate::config::AppConfig;
use actix_web::{HttpResponse, Responder, Scope, get, post, web};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rand::RngExt;
use rand_distr::Alphanumeric;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

pub fn subroutes() -> Scope {
    web::scope("/admin")
        .service(create_author_page)
        .service(create_author)
}

#[get("")]
async fn create_author_page() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><title>Create Author</title></head>
<body>
    <h1>Create Author</h1>
    <form action="/admin/create-author" method="post">
        <label>Author name <input type="text" name="name" required></label>
        <label>Admin token <input type="password" name="admin_token" required></label>
        <button type="submit">Create Author</button>
    </form>
</body>
</html>"#,
        )
}

#[derive(Deserialize)]
struct CreateAuthorForm {
    name: String,
    admin_token: String,
}

#[post("/create-author")]
async fn create_author(
    form: web::Form<CreateAuthorForm>,
    db: web::Data<Pool<SqliteConnectionManager>>,
    config: web::Data<AppConfig>,
) -> impl Responder {
    if config.admin_token.chars().count() <= 10 || form.admin_token != config.admin_token {
        return HttpResponse::Unauthorized().finish();
    }

    let name = form.name.trim();
    if name.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "error": "Author name is required"
        }));
    }

    let conn = match db.get() {
        Ok(conn) => conn,
        Err(error) => {
            eprintln!("[DATABASE ERROR] Failed to get connection to create author: {error}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let author_id = Uuid::new_v4().to_string();
    let author_secret = generate_author_secret();
    let result = conn
        .prepare("INSERT INTO authors (id, name, author_secret) VALUES (?1, ?2, ?3)")
        .and_then(|mut stmt| stmt.execute([&author_id, name, &author_secret]));

    match result {
        Ok(_) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(format!(
                r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><title>Author Created</title></head>
<body>
    <h1>Author created</h1>
    <p>Save this author secret; it is needed to sign in and will not be shown again.</p>
    <p><code>{author_secret}</code></p>
    <p><a href="/admin">Create another author</a></p>
</body>
</html>"#
            )),
        Err(error) => {
            eprintln!("[DATABASE ERROR] Failed to create author: {error}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

fn generate_author_secret() -> String {
    let mut rng = rand::rng();
    (0..64).map(|_| rng.sample(Alphanumeric) as char).collect()
}

#[cfg(test)]
mod tests {
    use super::generate_author_secret;

    #[test]
    fn author_secret_is_64_alphanumeric_characters() {
        let secret = generate_author_secret();

        assert_eq!(secret.len(), 64);
        assert!(
            secret
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        );
    }
}
