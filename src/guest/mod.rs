use crate::auth::is_authenticated_as_author;
use crate::db::Guest;
use actix_web::{HttpRequest, HttpResponse, Responder, Scope, delete, get, post, web};
use r2d2_sqlite::SqliteConnectionManager;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[get("")]
async fn get_guests(
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    // Check authentication
    if let Some(author_id) = is_authenticated_as_author(&req, &pool) {
        let conn = pool.get().unwrap();

        // Get all guests for this author
        let guests_result = conn
            .prepare("SELECT id, name, author FROM guests WHERE author = ?1 ORDER BY name")
            .and_then(|mut stmt| {
                let guest_iter = stmt.query_map([&author_id], Guest::from_row)?;

                let mut guests = Vec::new();
                for guest_result in guest_iter {
                    if let Ok(guest) = guest_result {
                        guests.push(guest.to_json());
                    }
                }
                Ok(guests)
            });

        match guests_result {
            Ok(guests) => HttpResponse::Ok().json(guests),
            Err(_) => {
                HttpResponse::InternalServerError().json(json!({"error": "Failed to fetch guests"}))
            }
        }
    } else {
        HttpResponse::Unauthorized().json(json!({"error": "Authentication required"}))
    }
}

#[derive(Deserialize)]
struct UpdateGuestForm {
    name: String,
}

#[get("/{guest_id}")]
async fn get_guest_details(
    path: web::Path<String>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let guest_id = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated_as_author(&req, &pool) {
        let conn = pool.get().unwrap();

        // Get guest details, ensuring it belongs to the authenticated author
        let guest = match conn
            .prepare("SELECT id, name, author FROM guests WHERE id = ?1 AND author = ?2")
            .and_then(|mut stmt| stmt.query_row([&guest_id, &author_id], Guest::from_row))
        {
            Ok(guest) => guest,
            Err(_) => {
                return HttpResponse::NotFound().json(json!({
                    "error": "Guest not found or access denied"
                }));
            }
        };

        HttpResponse::Ok().json(guest.to_json())
    } else {
        HttpResponse::Unauthorized().json(json!({"error": "Authentication required"}))
    }
}

#[post("/{guest_id}/update")]
async fn update_guest(
    path: web::Path<String>,
    form: web::Json<UpdateGuestForm>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let guest_id = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated_as_author(&req, &pool) {
        let conn = pool.get().unwrap();

        // Update guest name, ensuring it belongs to the authenticated author
        let result = conn
            .prepare("UPDATE guests SET name = ?1 WHERE id = ?2 AND author = ?3")
            .and_then(|mut stmt| stmt.execute([&form.name, &guest_id, &author_id]));

        match result {
            Ok(rows_affected) => {
                if rows_affected > 0 {
                    HttpResponse::Ok().json(json!({
                        "status": "success",
                        "message": "Guest updated successfully"
                    }))
                } else {
                    HttpResponse::NotFound().json(json!({
                        "error": "Guest not found or access denied"
                    }))
                }
            }
            Err(e) => {
                eprintln!("Database error updating guest: {}", e);
                HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to update guest"
                }))
            }
        }
    } else {
        HttpResponse::Unauthorized().json(json!({"error": "Authentication required"}))
    }
}

#[delete("/{guest_id}/delete")]
async fn delete_guest(
    path: web::Path<String>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let guest_id = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated_as_author(&req, &pool) {
        let conn = pool.get().unwrap();

        // Start a transaction to delete guest and related invitations
        let tx = conn.unchecked_transaction().unwrap();

        // Delete all invitations for this guest first
        let delete_invitations_result = tx
            .prepare("DELETE FROM invitations WHERE guest_id = ?1")
            .and_then(|mut stmt| stmt.execute([&guest_id]));

        // Delete the guest
        let delete_guest_result = tx
            .prepare("DELETE FROM guests WHERE id = ?1 AND author = ?2")
            .and_then(|mut stmt| stmt.execute([&guest_id, &author_id]));

        match (delete_invitations_result, delete_guest_result) {
            (Ok(_), Ok(rows_affected)) => {
                if rows_affected > 0 {
                    tx.commit().unwrap();
                    HttpResponse::Ok().json(json!({
                        "status": "success",
                        "message": "Guest deleted successfully"
                    }))
                } else {
                    tx.rollback().unwrap();
                    HttpResponse::NotFound().json(json!({
                        "error": "Guest not found or access denied"
                    }))
                }
            }
            _ => {
                tx.rollback().unwrap();
                HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to delete guest"
                }))
            }
        }
    } else {
        HttpResponse::Unauthorized().json(json!({"error": "Authentication required"}))
    }
}

#[post("/new")]
async fn create_guest(
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    // Check authentication
    if let Some(author_id) = is_authenticated_as_author(&req, &pool) {
        let conn = pool.get().unwrap();

        // Create new guest ID
        let guest_id = Uuid::new_v4().to_string();

        // Create empty guest with default values
        let default_name = "New Guest";

        let result = conn
            .prepare("INSERT INTO guests (id, name, author) VALUES (?1, ?2, ?3)")
            .and_then(|mut stmt| stmt.execute([&guest_id, default_name, &author_id]));

        match result {
            Ok(_) => HttpResponse::Ok().json(json!({
                "status": "success",
                "message": "Guest created successfully",
                "guest_id": guest_id
            })),
            Err(e) => {
                eprintln!("Database error creating guest: {}", e);
                HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to create guest"
                }))
            }
        }
    } else {
        HttpResponse::Unauthorized().json(json!({
            "error": "Authentication required"
        }))
    }
}

pub fn subroutes() -> Scope {
    web::scope("/guest")
        .service(get_guests)
        .service(create_guest)
        .service(get_guest_details)
        .service(update_guest)
        .service(delete_guest)
}
