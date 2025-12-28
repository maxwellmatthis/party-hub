use crate::auth::is_authenticated_as_author;
use crate::db::Party;
use crate::detect_language;
use actix_web::{HttpRequest, HttpResponse, Responder, Scope, delete, get, post, web};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::Deserialize;
use serde_json::json;
use std::fs;
use uuid::Uuid;

#[get("/")]
pub async fn manage(
    req: actix_web::HttpRequest,
    db: web::Data<Pool<SqliteConnectionManager>>,
) -> impl Responder {
    // Check if user is authenticated
    if is_authenticated_as_author(&req, &db).is_none() {
        // Redirect to auth page if not authenticated
        return HttpResponse::Found()
            .append_header(("Location", "/auth"))
            .finish();
    }

    // Detect language from Accept-Language header
    let language = detect_language(&req);

    let filename = match language.as_str() {
        "de" => "pages/de/manage_de.html",
        _ => "pages/en/manage_en.html", // Default to English
    };

    let html_content =
        fs::read_to_string(filename).unwrap_or_else(|_| "<h1>404: File Not Found</h1>".to_string());
    HttpResponse::Ok()
        .content_type("text/html")
        .body(html_content)
}

#[get("")]
async fn get_parties(
    req: actix_web::HttpRequest,
    db: web::Data<Pool<SqliteConnectionManager>>,
) -> impl Responder {
    // Check if user is authenticated
    let author_id = match is_authenticated_as_author(&req, &db) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized().json(json!({
                "error": "Authentication required"
            }));
        }
    };

    let conn = match db.get() {
        Ok(conn) => conn,
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "Database connection failed"
            }));
        }
    };

    // Get all parties for this author
    let parties_result = conn
        .prepare("SELECT id, name, author, invitation_blocks, date, respond_until, frozen, public, max_guests, has_rsvp_block FROM parties WHERE author = ?1")
        .and_then(|mut stmt| {
            let party_iter = stmt.query_map([&author_id], Party::from_row)?;

            let mut parties = Vec::new();
            for party_result in party_iter {
                if let Ok(party) = party_result {
                    parties.push(party.to_summary_json());
                }
            }
            Ok(parties)
        });

    match parties_result {
        Ok(parties) => HttpResponse::Ok().json(parties),
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to fetch parties"
            }))
        }
    }
}

#[post("/new")]
async fn create_party(
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    // Check authentication
    if let Some(author_id) = is_authenticated_as_author(&req, &pool) {
        let conn = pool.get().unwrap();

        // Create new party ID
        let party_id = Uuid::new_v4().to_string();

        // Create empty party with default values
        let default_name = "New Party";
        let default_invitation_blocks = "[]";
        let default_date = "";
        let default_respond_until = "";
        let default_frozen = false;
        let default_public = false;
        let default_max_guests = 0;
        let default_has_rsvp_block = false;

        let result = conn
            .prepare(
                "INSERT INTO parties (id, name, invitation_blocks, author, date, respond_until, frozen, public, max_guests, has_rsvp_block) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )
            .and_then(|mut stmt| {
                stmt.execute(rusqlite::params![
                    &party_id,
                    default_name,
                    default_invitation_blocks,
                    &author_id,
                    default_date,
                    default_respond_until,
                    default_frozen,
                    default_public,
                    default_max_guests,
                    default_has_rsvp_block,
                ])
            });

        match result {
            Ok(_) => HttpResponse::Ok().json(json!({
                "status": "success",
                "message": "Party created successfully",
                "party_id": party_id
            })),
            Err(e) => {
                eprintln!("Database error creating party: {}", e);
                HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to create party"
                }))
            }
        }
    } else {
        HttpResponse::Unauthorized().json(json!({
            "error": "Authentication required"
        }))
    }
}

#[get("/{party_id}")]
async fn get_party_details(
    path: web::Path<String>,
    req: actix_web::HttpRequest,
    db: web::Data<Pool<SqliteConnectionManager>>,
) -> impl Responder {
    let party_id = path.into_inner();

    // Check if user is authenticated
    let author_id = match is_authenticated_as_author(&req, &db) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized().json(json!({
                "error": "Authentication required"
            }));
        }
    };

    let conn = match db.get() {
        Ok(conn) => conn,
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "Database connection failed"
            }));
        }
    };

    // Get party details, ensuring it belongs to the authenticated author
    let party = match conn
        .prepare(
            "SELECT id, name, author, invitation_blocks, date, respond_until, frozen, public, max_guests, has_rsvp_block FROM parties WHERE id = ?1 AND author = ?2",
        )
        .and_then(|mut stmt| stmt.query_row([&party_id, &author_id], Party::from_row))
    {
        Ok(party) => party,
        Err(_) => {
            return HttpResponse::NotFound().json(json!({
                "error": "Party not found or access denied"
            }));
        }
    };

    // Parse invitation blocks JSON
    let invitation_blocks = party.get_invitation_blocks_json();

    // Get all guests for this party
    let guests_result = conn.prepare("SELECT g.id, g.salutation, g.first, g.last, i.organizer, i.id FROM guests g INNER JOIN invitations i ON g.id = i.guest_id WHERE i.party_id = ?1")
        .and_then(|mut stmt| {
            let guest_iter = stmt.query_map([&party_id], |row| {
                let guest_id: String = row.get(0)?;
                let salutation: String = row.get(1)?;
                let first: String = row.get(2)?;
                let last: String = row.get(3)?;
                let organizer: bool = row.get(4)?;
                let invitation_id: String = row.get(5)?;
                Ok(json!({
                    "id": guest_id,
                    "salutation": salutation,
                    "first": first,
                    "last": last,
                    "name": format!("{} {}", first, last).trim(),
                    "organizer": organizer,
                    "invitation_id": invitation_id
                }))
            })?;

            let mut guests = Vec::new();
            for guest_result in guest_iter {
                if let Ok(guest) = guest_result {
                    guests.push(guest);
                }
            }
            Ok(guests)
        });

    let guests = guests_result.unwrap_or_else(|_| Vec::new());

    let response = json!({
        "id": party.id,
        "name": party.name,
        "date": party.date,
        "respond_until": party.respond_until,
        "frozen": party.frozen,
        "public": party.public,
        "max_guests": party.max_guests,
        "has_rsvp_block": party.has_rsvp_block,
        "invitation_blocks": invitation_blocks,
        "guests": guests
    });

    HttpResponse::Ok().json(response)
}

#[derive(Deserialize)]
struct SavePartyForm {
    name: String,
    invitation_blocks: Option<String>,
    date: Option<String>,
    respond_until: Option<String>,
    frozen: Option<bool>,
    public: Option<bool>,
    max_guests: Option<i64>,
}

#[post("/{party_id}/update")]
async fn update_party(
    path: web::Path<String>,
    form: web::Json<SavePartyForm>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let party_id = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated_as_author(&req, &pool) {
        // Verify party ownership
        match verify_party_ownership(&pool, &party_id, &author_id) {
            Ok(true) => {
                let conn = pool.get().unwrap();
                let invitation_blocks = form.invitation_blocks.as_deref().unwrap_or("[]");
                let date = form.date.as_deref().unwrap_or("");
                let respond_until = form.respond_until.as_deref().unwrap_or("");
                let frozen = form.frozen.unwrap_or(false);
                let public = form.public.unwrap_or(false);
                let max_guests = form.max_guests.unwrap_or(0);

                // Validate attendance blocks: parse the JSON and count attendance blocks
                let has_rsvp_block = match serde_json::from_str::<Vec<serde_json::Value>>(invitation_blocks) {
                    Ok(blocks) => {
                        let attendance_count = blocks.iter()
                            .filter(|block| {
                                block.get("template")
                                    .and_then(|t| t.as_str())
                                    .map(|t| t == "attendance")
                                    .unwrap_or(false)
                            })
                            .count();

                        if attendance_count > 1 {
                            return HttpResponse::BadRequest().json(json!({
                                "error": "Only one attendance block is allowed per party"
                            }));
                        }

                        attendance_count == 1
                    }
                    Err(_) => {
                        return HttpResponse::BadRequest().json(json!({
                            "error": "Invalid invitation_blocks JSON"
                        }));
                    }
                };

                let result = conn
                    .prepare("UPDATE parties SET name = ?1, invitation_blocks = ?2, date = ?3, respond_until = ?4, frozen = ?5, public = ?6, max_guests = ?7, has_rsvp_block = ?8 WHERE id = ?9 AND author = ?10")
                    .and_then(|mut stmt| stmt.execute(rusqlite::params![&form.name, invitation_blocks, date, respond_until, frozen, public, max_guests, has_rsvp_block, &party_id, &author_id]));

                match result {
                    Ok(rows_affected) => {
                        if rows_affected > 0 {
                            HttpResponse::Ok().json(json!({
                                "status": "success",
                                "message": "Party updated successfully"
                            }))
                        } else {
                            HttpResponse::NotFound().json(json!({
                                "error": "Party not found"
                            }))
                        }
                    }
                    Err(e) => {
                        eprintln!("Database error updating party: {}", e);
                        HttpResponse::InternalServerError().json(json!({
                            "error": "Failed to update party"
                        }))
                    }
                }
            }
            Ok(false) => HttpResponse::Forbidden().json(json!({
                "error": "Party not found or access denied"
            })),
            Err(_) => HttpResponse::InternalServerError().json(json!({
                "error": "Database error"
            })),
        }
    } else {
        HttpResponse::Unauthorized().json(json!({
            "error": "Authentication required"
        }))
    }
}

#[delete("/{party_id}/delete")]
async fn delete_party(
    path: web::Path<String>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let party_id = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated_as_author(&req, &pool) {
        // Verify party ownership
        match verify_party_ownership(&pool, &party_id, &author_id) {
            Ok(true) => {
                let conn = pool.get().unwrap();

                // Start a transaction to delete party and related invitations
                let tx = conn.unchecked_transaction().unwrap();

                // Delete all invitations for this party first
                let delete_invitations_result = tx
                    .prepare("DELETE FROM invitations WHERE party_id = ?1")
                    .and_then(|mut stmt| stmt.execute([&party_id]));

                // Delete the party
                let delete_party_result = tx
                    .prepare("DELETE FROM parties WHERE id = ?1 AND author = ?2")
                    .and_then(|mut stmt| stmt.execute([&party_id, &author_id]));

                match (delete_invitations_result, delete_party_result) {
                    (Ok(_), Ok(rows_affected)) => {
                        if rows_affected > 0 {
                            tx.commit().unwrap();
                            HttpResponse::Ok().json(json!({
                                "status": "success",
                                "message": "Party deleted successfully"
                            }))
                        } else {
                            tx.rollback().unwrap();
                            HttpResponse::NotFound().json(json!({
                                "error": "Party not found"
                            }))
                        }
                    }
                    _ => {
                        tx.rollback().unwrap();
                        HttpResponse::InternalServerError().json(json!({
                            "error": "Failed to delete party"
                        }))
                    }
                }
            }
            Ok(false) => HttpResponse::Forbidden().json(json!({
                "error": "Party not found or access denied"
            })),
            Err(_) => HttpResponse::InternalServerError().json(json!({
                "error": "Database error"
            })),
        }
    } else {
        HttpResponse::Unauthorized().json(json!({
            "error": "Authentication required"
        }))
    }
}

fn verify_party_ownership(
    pool: &r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
    party_id: &str,
    author_id: &str,
) -> Result<bool, rusqlite::Error> {
    let conn = pool.get().unwrap();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM parties WHERE id = ?1 AND author = ?2",
        [party_id, author_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

#[post("/{party_id}/add/{guest_id}")]
async fn add_guest_to_party(
    path: web::Path<(String, String)>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let (party_id, guest_id) = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated_as_author(&req, &pool) {
        // Verify party ownership
        match verify_party_ownership(&pool, &party_id, &author_id) {
            Ok(true) => {
                let conn = pool.get().unwrap();
                // Check if guest exists and belongs to the author
                let guest_exists: Result<bool, _> = conn.query_row(
                    "SELECT COUNT(*) FROM guests WHERE id = ?1 AND author = ?2",
                    [&guest_id, &author_id],
                    |row| {
                        let count: i64 = row.get(0)?;
                        Ok(count > 0)
                    },
                );

                match guest_exists {
                    Ok(true) => {
                        // Check if invitation already exists
                        let invitation_exists: Result<bool, _> = conn.query_row(
                            "SELECT COUNT(*) FROM invitations WHERE guest_id = ?1 AND party_id = ?2",
                            [&guest_id, &party_id],
                            |row| {
                                let count: i64 = row.get(0)?;
                                Ok(count > 0)
                            }
                        );

                        match invitation_exists {
                            Ok(false) => {
                                // Create new invitation
                                let invitation_id = Uuid::new_v4().to_string();
                                let result = conn
                                    .prepare("INSERT INTO invitations (id, guest_id, party_id, invitation_block_answers, organizer) VALUES (?1, ?2, ?3, '{}', 0)")
                                    .and_then(|mut stmt| stmt.execute([&invitation_id, &guest_id, &party_id]));

                                match result {
                                    Ok(_) => HttpResponse::Ok().json(json!({"status": "success", "message": "Guest added to party"})),
                                    Err(_) => HttpResponse::InternalServerError().json(json!({"error": "Failed to add guest to party"}))
                                }
                            }
                            Ok(true) => HttpResponse::Conflict()
                                .json(json!({"error": "Guest is already invited to this party"})),
                            Err(_) => HttpResponse::InternalServerError()
                                .json(json!({"error": "Database error"})),
                        }
                    }
                    Ok(false) => HttpResponse::NotFound()
                        .json(json!({"error": "Guest not found or does not belong to you"})),
                    Err(_) => {
                        HttpResponse::InternalServerError().json(json!({"error": "Database error"}))
                    }
                }
            }
            Ok(false) => {
                HttpResponse::Forbidden().json(json!({"error": "Party not found or access denied"}))
            }
            Err(_) => HttpResponse::InternalServerError().json(json!({"error": "Database error"})),
        }
    } else {
        HttpResponse::Unauthorized().json(json!({"error": "Authentication required"}))
    }
}

#[delete("/{party_id}/remove/{guest_id}")]
async fn remove_guest_from_party(
    path: web::Path<(String, String)>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let (party_id, guest_id) = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated_as_author(&req, &pool) {
        // Verify party ownership
        match verify_party_ownership(&pool, &party_id, &author_id) {
            Ok(true) => {
                let conn = pool.get().unwrap();
                // Remove invitation
                let result = conn
                    .prepare("DELETE FROM invitations WHERE guest_id = ?1 AND party_id = ?2")
                    .and_then(|mut stmt| stmt.execute([&guest_id, &party_id]));

                match result {
                    Ok(rows_affected) => {
                        if rows_affected > 0 {
                            HttpResponse::Ok().json(
                                json!({"status": "success", "message": "Guest removed from party"}),
                            )
                        } else {
                            HttpResponse::NotFound()
                                .json(json!({"error": "Guest invitation not found"}))
                        }
                    }
                    Err(_) => HttpResponse::InternalServerError()
                        .json(json!({"error": "Failed to remove guest from party"})),
                }
            }
            Ok(false) => {
                HttpResponse::Forbidden().json(json!({"error": "Party not found or access denied"}))
            }
            Err(_) => HttpResponse::InternalServerError().json(json!({"error": "Database error"})),
        }
    } else {
        HttpResponse::Unauthorized().json(json!({"error": "Authentication required"}))
    }
}

#[post("/{party_id}/promote/{guest_id}")]
async fn promote_guest_to_organizer(
    path: web::Path<(String, String)>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let (party_id, guest_id) = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated_as_author(&req, &pool) {
        // Verify party ownership
        match verify_party_ownership(&pool, &party_id, &author_id) {
            Ok(true) => {
                let conn = pool.get().unwrap();
                // Update invitation to set organizer = true
                let result = conn
                    .prepare("UPDATE invitations SET organizer = 1 WHERE guest_id = ?1 AND party_id = ?2")
                    .and_then(|mut stmt| stmt.execute([&guest_id, &party_id]));

                match result {
                    Ok(rows_affected) => {
                        if rows_affected > 0 {
                            HttpResponse::Ok().json(json!({"status": "success", "message": "Guest promoted to organizer"}))
                        } else {
                            HttpResponse::NotFound()
                                .json(json!({"error": "Guest invitation not found"}))
                        }
                    }
                    Err(_) => HttpResponse::InternalServerError()
                        .json(json!({"error": "Failed to promote guest"})),
                }
            }
            Ok(false) => {
                HttpResponse::Forbidden().json(json!({"error": "Party not found or access denied"}))
            }
            Err(_) => HttpResponse::InternalServerError().json(json!({"error": "Database error"})),
        }
    } else {
        HttpResponse::Unauthorized().json(json!({"error": "Authentication required"}))
    }
}

#[post("/{party_id}/demote/{guest_id}")]
async fn demote_organizer_to_guest(
    path: web::Path<(String, String)>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let (party_id, guest_id) = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated_as_author(&req, &pool) {
        // Verify party ownership
        match verify_party_ownership(&pool, &party_id, &author_id) {
            Ok(true) => {
                let conn = pool.get().unwrap();
                // Update invitation to set organizer = false
                let result = conn
                    .prepare("UPDATE invitations SET organizer = 0 WHERE guest_id = ?1 AND party_id = ?2")
                    .and_then(|mut stmt| stmt.execute([&guest_id, &party_id]));

                match result {
                    Ok(rows_affected) => {
                        if rows_affected > 0 {
                            HttpResponse::Ok().json(json!({"status": "success", "message": "Organizer demoted to guest"}))
                        } else {
                            HttpResponse::NotFound()
                                .json(json!({"error": "Guest invitation not found"}))
                        }
                    }
                    Err(_) => HttpResponse::InternalServerError()
                        .json(json!({"error": "Failed to demote organizer"})),
                }
            }
            Ok(false) => {
                HttpResponse::Forbidden().json(json!({"error": "Party not found or access denied"}))
            }
            Err(_) => HttpResponse::InternalServerError().json(json!({"error": "Database error"})),
        }
    } else {
        HttpResponse::Unauthorized().json(json!({"error": "Authentication required"}))
    }
}

pub fn subroutes() -> Scope {
    web::scope("/party")
        .service(create_party)
        .service(update_party)
        .service(delete_party)
        .service(get_parties)
        .service(get_party_details)
        .service(add_guest_to_party)
        .service(remove_guest_from_party)
        .service(promote_guest_to_organizer)
        .service(demote_organizer_to_guest)
}
