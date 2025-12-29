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
            .prepare("SELECT id, salutation, first, last, email, note, author, selfcreated FROM guests WHERE author = ?1 ORDER BY last, first")
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
    salutation: String,
    first: String,
    last: String,
    email: String,
    note: String,
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
            .prepare("SELECT id, salutation, first, last, email, note, author, selfcreated FROM guests WHERE id = ?1 AND author = ?2")
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

        // Update guest fields, ensuring it belongs to the authenticated author
        let result = conn
            .prepare("UPDATE guests SET salutation = ?1, first = ?2, last = ?3, email = ?4, note = ?5 WHERE id = ?6 AND author = ?7")
            .and_then(|mut stmt| stmt.execute([&form.salutation, &form.first, &form.last, &form.email, &form.note, &guest_id, &author_id]));

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
        let default_salutation = "";
        let default_first = "New";
        let default_last = "";
        let default_email = "";
        let default_note = "";

        let result = conn
            .prepare("INSERT INTO guests (id, salutation, first, last, email, note, author) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
            .and_then(|mut stmt| stmt.execute([&guest_id, default_salutation, default_first, default_last, default_email, default_note, &author_id]));

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

#[derive(Deserialize)]
struct CreatePublicGuestForm {
    salutation: String,
    first: String,
    last: String,
    email: String,
}

#[post("/public_guest/{party_id}")]
async fn create_public_guest(
    path: web::Path<String>,
    form: web::Json<CreatePublicGuestForm>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let party_id = path.into_inner();
    let language = crate::detect_language(&req);

    let conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().json(json!({
            "error": "Database connection failed"
        })),
    };

    // Verify party exists and is public
    let (author_id, max_guests) = match conn
        .prepare("SELECT author, public, max_guests FROM parties WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&party_id], |row| {
                let author: String = row.get(0)?;
                let is_public: bool = row.get(1)?;
                let max_guests: i64 = row.get(2)?;
                Ok((author, is_public, max_guests))
            })
        }) {
        Ok((author, is_public, max_guests)) => {
            if !is_public {
                let error_msg = match language.as_str() {
                    "de" => "Diese Party ist nicht öffentlich",
                    _ => "This party is not public"
                };
                return HttpResponse::Forbidden().json(json!({
                    "error": error_msg
                }));
            }
            (author, max_guests)
        }
        Err(_) => {
            let error_msg = match language.as_str() {
                "de" => "Party nicht gefunden",
                _ => "Party not found"
            };
            return HttpResponse::NotFound().json(json!({
                "error": error_msg
            }));
        }
    };

    // Check max guests limit if set
    // Only count guests who have RSVP'd "yes" (attendance answer = 0)
    if max_guests > 0 {
        let guest_count = conn
            .prepare("SELECT invitation_blocks FROM parties WHERE id = ?1")
            .and_then(|mut stmt| {
                stmt.query_row([&party_id], |row| {
                    let invitation_blocks: String = row.get(0)?;
                    Ok(invitation_blocks)
                })
            })
            .ok()
            .and_then(|blocks_json| {
                // Parse invitation blocks to find attendance block ID
                serde_json::from_str::<serde_json::Value>(&blocks_json)
                    .ok()
                    .and_then(|blocks| {
                        blocks.as_array().and_then(|arr| {
                            arr.iter().find_map(|block| {
                                if block.get("template")?.as_str()? == "attendance" {
                                    block.get("id")?.as_str().map(String::from)
                                } else {
                                    None
                                }
                            })
                        })
                    })
            });

        let yes_count = if let Some(attendance_id) = guest_count {
            // Count only guests who answered "yes" (0) to the attendance block
            conn.prepare("SELECT invitation_block_answers FROM invitations WHERE party_id = ?1")
                .and_then(|mut stmt| {
                    let answer_iter = stmt.query_map([&party_id], |row| {
                        let answers: String = row.get(0)?;
                        Ok(answers)
                    })?;

                    let mut count = 0i64;
                    for answer_result in answer_iter {
                        if let Ok(answer_str) = answer_result {
                            if let Ok(answer_json) = serde_json::from_str::<serde_json::Value>(&answer_str) {
                                if let Some(attendance_answer) = answer_json.get(&attendance_id) {
                                    if let Some(answer_val) = attendance_answer.as_i64() {
                                        if answer_val == 0 {
                                            count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(count)
                })
                .unwrap_or(0)
        } else {
            // No attendance block - count all invitations
            conn.prepare("SELECT COUNT(*) FROM invitations WHERE party_id = ?1")
                .and_then(|mut stmt| {
                    stmt.query_row([&party_id], |row| {
                        let count: i64 = row.get(0)?;
                        Ok(count)
                    })
                })
                .unwrap_or(0)
        };

        if yes_count >= max_guests {
            let error_msg = match language.as_str() {
                "de" => "Diese Party hat die maximale Anzahl an Gästen erreicht",
                _ => "This party has reached its maximum number of guests"
            };
            return HttpResponse::Forbidden().json(json!({
                "error": error_msg
            }));
        }
    }

    // Create new guest with selfcreated=true
    let guest_id = Uuid::new_v4().to_string();
    let invitation_id = Uuid::new_v4().to_string();

    let guest_result = conn
        .prepare("INSERT INTO guests (id, salutation, first, last, email, note, author, selfcreated) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)")
        .and_then(|mut stmt| {
            stmt.execute([
                &guest_id,
                &form.salutation,
                &form.first,
                &form.last,
                &form.email,
                "",  // empty note
                &author_id,
                "1", // selfcreated = true
            ])
        });

    if let Err(e) = guest_result {
        eprintln!("Database error creating public guest: {}", e);
        return HttpResponse::InternalServerError().json(json!({
            "error": "Failed to create guest"
        }));
    }

    // Create invitation for the guest (without answers - they'll be saved via save_answers endpoint)
    let invitation_result = conn
        .prepare("INSERT INTO invitations (id, guest_id, party_id, invitation_block_answers, organizer) VALUES (?1, ?2, ?3, ?4, ?5)")
        .and_then(|mut stmt| {
            stmt.execute([
                &invitation_id,
                &guest_id,
                &party_id,
                "", // empty answers - will be populated by save_answers endpoint
                "0", // organizer = false
            ])
        });

    match invitation_result {
        Ok(_) => HttpResponse::Ok().json(json!({
            "status": "success",
            "message": "Guest created successfully",
            "guest_id": guest_id,
            "invitation_id": invitation_id
        })),
        Err(e) => {
            eprintln!("Database error creating invitation: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to create invitation"
            }))
        }
    }
}

pub fn subroutes() -> Scope {
    web::scope("/guest")
        .service(get_guests)
        .service(create_guest)
        .service(get_guest_details)
        .service(update_guest)
        .service(delete_guest)
        .service(create_public_guest)
}
