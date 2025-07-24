use actix_web::{
    App, HttpRequest, HttpResponse, HttpServer, Responder,
    cookie::{Cookie, SameSite},
    delete, get, post, web,
};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::fs;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

mod db;
use db::{prepare_db, Party, Invitation, Guest, Author};

// -- Statically Served --

#[get("/static/{filename}")]
async fn serve_static(path: web::Path<String>) -> impl Responder {
    let filename = path.into_inner();

    // Whitelist of allowed static files for security
    let allowed_files = vec![
        "auth.css",
        "chevrons-down.svg",
        "chevrons-up.svg",
        "chevron-down.svg",
        "chevron-up.svg",
        "clipboard.svg",
        "invitation.css",
        "invitation.js",
        "manage.css",
        "manage.js",
        "menu.svg",
        "plus.svg",
        "trash-2.svg",
        "favicon.ico",
        "x.svg",
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

// -- Guest Facing Routes --

#[get("/{invitation_id}")]
async fn invitation_page(
    path: web::Path<String>,
    req: actix_web::HttpRequest,
    db: web::Data<Pool<SqliteConnectionManager>>,
) -> impl Responder {
    let invitation_id = path.into_inner();
    
    // Verify this is actually a valid invitation ID before serving the page
    let conn = match db.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("Database connection failed"),
    };
    
    // Check if invitation exists
    let invitation_exists = conn
        .prepare("SELECT COUNT(*) FROM invitations WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&invitation_id], |row| {
                let count: i32 = row.get(0)?;
                Ok(count > 0)
            })
        })
        .unwrap_or(false);
    
    if !invitation_exists {
        return HttpResponse::NotFound().body("Invitation not found");
    }
    
    // Detect language from Accept-Language header
    let language = detect_language(&req);

    let filename = match language.as_str() {
        "de" => "pages/de/invitation_de.html",
        _ => "pages/en/invitation_en.html", // Default to English
    };

    let html_content =
        fs::read_to_string(filename).unwrap_or_else(|_| "<h1>404: File Not Found</h1>".to_string());
    HttpResponse::Ok()
        .content_type("text/html")
        .body(html_content)
}

#[get("/details/{invitation_id}")]
async fn details(
    path: web::Path<String>,
    db: web::Data<Pool<SqliteConnectionManager>>,
) -> impl Responder {
    let invitation_id = path.into_inner();

    let conn = match db.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("Database connection failed"),
    };

    let invitation = match conn
        .prepare("SELECT id, guest_id, party_id, invitation_block_answers, organizer FROM invitations WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&invitation_id], Invitation::from_row)
        }) {
        Ok(invitation) => invitation,
        Err(_) => return HttpResponse::BadRequest().body("Invitation not found"),
    };

    // Get guest name
    let guest_name = match conn
        .prepare("SELECT name FROM guests WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&invitation.guest_id], |row| {
                let name: String = row.get(0)?;
                Ok(name)
            })
        }) {
        Ok(name) => name,
        Err(_) => return HttpResponse::InternalServerError().body("Guest not found"),
    };

    let invitation_blocks = match conn
        .prepare("SELECT invitation_blocks FROM parties WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&invitation.party_id], |row| {
                let invitation_blocks: String = row.get(0)?;
                Ok(invitation_blocks)
            })
        }) {
        Ok(invitation_blocks) => invitation_blocks,
        Err(_) => return HttpResponse::InternalServerError().body("Party data not found"),
    };

    // Get all other guests' answers for the same party (excluding current invitation)
    // Include guest names for organizer view
    let all_other_answers = match conn.prepare("SELECT i.invitation_block_answers, g.name FROM invitations i JOIN guests g ON i.guest_id = g.id WHERE i.party_id = ?1 AND i.id != ?2 AND i.invitation_block_answers != ''")
        .and_then(|mut stmt| {
            let answer_iter = stmt.query_map([&invitation.party_id, &invitation_id], |row| {
                let answers: String = row.get(0)?;
                let guest_name: String = row.get(1)?;
                Ok((answers, guest_name))
            })?;

            let mut all_answers = Vec::new();
            for answer_result in answer_iter {
                if let Ok((answer_str, guest_name)) = answer_result {
                    if let Ok(answer_json) = serde_json::from_str::<serde_json::Value>(&answer_str) {
                        all_answers.push((answer_json, guest_name));
                    }
                }
            }
            Ok(all_answers)
        }) {
        Ok(answers) => answers,
        Err(_) => Vec::new(),
    };

    // Parse invitation blocks to determine which are public
    let blocks_json =
        serde_json::from_str::<serde_json::Value>(&invitation_blocks).unwrap_or(json!([]));
    let mut public_block_ids = std::collections::HashSet::new();

    if let Some(blocks_array) = blocks_json.as_array() {
        for block in blocks_array.iter() {
            // Get the block ID
            if let Some(block_id) = block.get("id").and_then(|v| v.as_str()) {
                if let Some(content) = block.get("content") {
                    // Try to parse content as JSON to check for public flag
                    if let Ok(content_obj) =
                        serde_json::from_str::<serde_json::Value>(content.as_str().unwrap_or("{}"))
                    {
                        if content_obj
                            .get("public")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false)
                        {
                            public_block_ids.insert(block_id.to_string());
                        }
                    }
                }
            }
        }
    }

    // Filter other guests' answers based on organizer status and visibility
    let filtered_other_answers: Vec<serde_json::Value> = if invitation.organizer {
        // Organizers can see all answers with guest names
        all_other_answers
            .into_iter()
            .map(|(guest_answers, guest_name)| {
                let mut answer_with_names = serde_json::Map::new();
                if let Some(answers_obj) = guest_answers.as_object() {
                    for (block_id, answer) in answers_obj {
                        // Create answer object with guest name
                        let answer_with_name = json!({
                            "answer": answer,
                            "guest_name": guest_name
                        });
                        answer_with_names.insert(block_id.clone(), answer_with_name);
                    }
                }
                serde_json::Value::Object(answer_with_names)
            })
            .collect()
    } else {
        // Regular guests can only see public answers with guest names
        all_other_answers
            .into_iter()
            .map(|(guest_answers, guest_name)| {
                let mut filtered_guest = serde_json::Map::new();
                if let Some(answers_obj) = guest_answers.as_object() {
                    for (block_id, answer) in answers_obj {
                        if public_block_ids.contains(block_id) {
                            // Create answer object with guest name for public answers
                            let answer_with_name = json!({
                                "answer": answer,
                                "guest_name": guest_name
                            });
                            filtered_guest.insert(block_id.clone(), answer_with_name);
                        }
                    }
                }
                serde_json::Value::Object(filtered_guest)
            })
            .collect()
    };

    let response = json!({
        "invitation_blocks": serde_json::from_str::<serde_json::Value>(&invitation_blocks).unwrap_or(json!([])),
        "invitation_block_answers": invitation.get_answers_json(),
        "other_guests_answers": filtered_other_answers,
        "guest_name": guest_name,
        "is_organizer": invitation.organizer,
    });

    HttpResponse::Ok()
        .content_type("application/json")
        .body(response.to_string())
}

#[derive(Deserialize)]
struct AuthForm {
    #[serde(rename = "author-secret")]
    author_secret: String,
}

#[derive(Deserialize)]
struct SavePartyForm {
    name: String,
    invitation_blocks: Option<String>,
}

#[derive(Deserialize)]
struct SaveAnswersRequest {
    answers: serde_json::Value,
}

#[post("/details/{invitation_id}")]
async fn save_answers(
    path: web::Path<String>,
    json: web::Json<SaveAnswersRequest>,
    db: web::Data<Pool<SqliteConnectionManager>>,
) -> impl Responder {
    let invitation_id = path.into_inner();
    let answers = &json.answers;

    let conn = match db.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("Database connection failed"),
    };

    // Get invitation and party information
    let party_id = match conn
        .prepare("SELECT party_id FROM invitations WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&invitation_id], |row| {
                let party_id: String = row.get(0)?;
                Ok(party_id)
            })
        }) {
        Ok(party_id) => party_id,
        Err(_) => return HttpResponse::BadRequest().json(json!({
            "error": "Invitation not found"
        })),
    };

    // Get valid block IDs from the party's invitation blocks
    let valid_block_ids = match conn
        .prepare("SELECT invitation_blocks FROM parties WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&party_id], |row| {
                let invitation_blocks: String = row.get(0)?;
                Ok(invitation_blocks)
            })
        }) {
        Ok(invitation_blocks) => {
            // Parse invitation blocks to extract valid block IDs
            let blocks_json = serde_json::from_str::<serde_json::Value>(&invitation_blocks)
                .unwrap_or(json!([]));
            
            let mut valid_ids = std::collections::HashSet::new();
            if let Some(blocks_array) = blocks_json.as_array() {
                for block in blocks_array.iter() {
                    if let Some(block_id) = block.get("id").and_then(|v| v.as_str()) {
                        valid_ids.insert(block_id.to_string());
                    }
                }
            }
            valid_ids
        },
        Err(_) => return HttpResponse::InternalServerError().json(json!({
            "error": "Failed to get party data"
        })),
    };

    // Filter answers to only include valid block IDs
    let filtered_answers = if let Some(answers_obj) = answers.as_object() {
        let mut filtered = serde_json::Map::new();
        for (block_id, answer) in answers_obj {
            if valid_block_ids.contains(block_id) {
                filtered.insert(block_id.clone(), answer.clone());
            }
        }
        serde_json::Value::Object(filtered)
    } else {
        json!({})
    };

    // Convert filtered answers to JSON string
    let answers_json = filtered_answers.to_string();

    // Update the invitation with filtered answers using prepared statement
    let update_result = conn
        .prepare("UPDATE invitations SET invitation_block_answers = ?1 WHERE id = ?2")
        .and_then(|mut stmt| stmt.execute([&answers_json, &invitation_id]));

    match update_result {
        Ok(_) => HttpResponse::Ok().json(json!({
            "success": true,
            "message": "Answers saved successfully"
        })),
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to save answers"
            }))
        }
    }
}

// -- Authentication --

#[get("/auth")]
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

#[post("/auth")]
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
        .and_then(|mut stmt| {
            stmt.query_row([&form.author_secret], Author::from_row)
        });

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

// -- Authenticated Routes --

fn is_authenticated(
    req: &actix_web::HttpRequest,
    db: &Pool<SqliteConnectionManager>,
) -> Option<String> {
    if let Some(cookie) = req.cookie("auth_token") {
        let auth_secret = cookie.value();

        // Validate the token against the database
        if let Ok(conn) = db.get() {
            let author_result = conn
                .prepare("SELECT id, name, author_secret FROM authors WHERE author_secret = ?1")
                .and_then(|mut stmt| {
                    stmt.query_row([auth_secret], Author::from_row)
                });

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

#[get("/")]
async fn manage(
    req: actix_web::HttpRequest,
    db: web::Data<Pool<SqliteConnectionManager>>,
) -> impl Responder {
    // Check if user is authenticated
    if is_authenticated(&req, &db).is_none() {
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

#[get("/party")]
async fn get_parties(
    req: actix_web::HttpRequest,
    db: web::Data<Pool<SqliteConnectionManager>>,
) -> impl Responder {
    // Check if user is authenticated
    let author_id = match is_authenticated(&req, &db) {
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
        .prepare("SELECT id, name, author, invitation_blocks FROM parties WHERE author = ?1")
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

#[post("/party/new")]
async fn create_party(
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    // Check authentication
    if let Some(author_id) = is_authenticated(&req, &pool) {
        let conn = pool.get().unwrap();

        // Create new party ID
        let party_id = Uuid::new_v4().to_string();
        
        // Create empty party with default values
        let default_name = "New Party";
        let default_invitation_blocks = "[]";

        let result = conn
            .prepare("INSERT INTO parties (id, name, invitation_blocks, author) VALUES (?1, ?2, ?3, ?4)")
            .and_then(|mut stmt| stmt.execute([&party_id, default_name, default_invitation_blocks, &author_id]));

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

#[get("/party/{party_id}")]
async fn get_party_details(
    path: web::Path<String>,
    req: actix_web::HttpRequest,
    db: web::Data<Pool<SqliteConnectionManager>>,
) -> impl Responder {
    let party_id = path.into_inner();

    // Check if user is authenticated
    let author_id = match is_authenticated(&req, &db) {
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
        .prepare("SELECT id, name, author, invitation_blocks FROM parties WHERE id = ?1 AND author = ?2")
        .and_then(|mut stmt| {
            stmt.query_row([&party_id, &author_id], Party::from_row)
        }) {
        Ok(party) => party,
        Err(_) => return HttpResponse::NotFound().json(json!({
            "error": "Party not found or access denied"
        })),
    };

    // Parse invitation blocks JSON
    let invitation_blocks = party.get_invitation_blocks_json();

    // Get all guests for this party
    let guests_result = conn.prepare("SELECT g.id, g.name, i.organizer, i.id FROM guests g INNER JOIN invitations i ON g.id = i.guest_id WHERE i.party_id = ?1")
        .and_then(|mut stmt| {
            let guest_iter = stmt.query_map([&party_id], |row| {
                let guest_id: String = row.get(0)?;
                let guest_name: String = row.get(1)?;
                let organizer: bool = row.get(2)?;
                let invitation_id: String = row.get(3)?;
                Ok(json!({
                    "id": guest_id,
                    "name": guest_name,
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
        "invitation_blocks": invitation_blocks,
        "guests": guests
    });

    HttpResponse::Ok().json(response)
}

#[post("/party/{party_id}/update")]
async fn update_party(
    path: web::Path<String>,
    form: web::Json<SavePartyForm>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let party_id = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated(&req, &pool) {
        // Verify party ownership
        match verify_party_ownership(&pool, &party_id, &author_id) {
            Ok(true) => {
                let conn = pool.get().unwrap();
                let invitation_blocks = form.invitation_blocks.as_deref().unwrap_or("[]");

                let result = conn
                    .prepare("UPDATE parties SET name = ?1, invitation_blocks = ?2 WHERE id = ?3 AND author = ?4")
                    .and_then(|mut stmt| stmt.execute([&form.name, invitation_blocks, &party_id, &author_id]));

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

#[delete("/party/{party_id}/delete")]
async fn delete_party(
    path: web::Path<String>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let party_id = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated(&req, &pool) {
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

#[post("/party/{party_id}/add/{guest_id}")]
async fn add_guest_to_party(
    path: web::Path<(String, String)>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let (party_id, guest_id) = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated(&req, &pool) {
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

#[delete("/party/{party_id}/remove/{guest_id}")]
async fn remove_guest_from_party(
    path: web::Path<(String, String)>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let (party_id, guest_id) = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated(&req, &pool) {
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

#[post("/party/{party_id}/promote/{guest_id}")]
async fn promote_guest_to_organizer(
    path: web::Path<(String, String)>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let (party_id, guest_id) = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated(&req, &pool) {
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

#[post("/party/{party_id}/demote/{guest_id}")]
async fn demote_organizer_to_guest(
    path: web::Path<(String, String)>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let (party_id, guest_id) = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated(&req, &pool) {
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

#[get("/guest")]
async fn get_guests(
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    // Check authentication
    if let Some(author_id) = is_authenticated(&req, &pool) {
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

#[get("/guest/{guest_id}")]
async fn get_guest_details(
    path: web::Path<String>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let guest_id = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated(&req, &pool) {
        let conn = pool.get().unwrap();

        // Get guest details, ensuring it belongs to the authenticated author
        let guest = match conn
            .prepare("SELECT id, name, author FROM guests WHERE id = ?1 AND author = ?2")
            .and_then(|mut stmt| {
                stmt.query_row([&guest_id, &author_id], Guest::from_row)
            }) {
            Ok(guest) => guest,
            Err(_) => return HttpResponse::NotFound().json(json!({
                "error": "Guest not found or access denied"
            })),
        };

        HttpResponse::Ok().json(guest.to_json())
    } else {
        HttpResponse::Unauthorized().json(json!({"error": "Authentication required"}))
    }
}

#[post("/guest/{guest_id}/update")]
async fn update_guest(
    path: web::Path<String>,
    form: web::Json<UpdateGuestForm>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let guest_id = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated(&req, &pool) {
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

#[delete("/guest/{guest_id}/delete")]
async fn delete_guest(
    path: web::Path<String>,
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    let guest_id = path.into_inner();

    // Check authentication
    if let Some(author_id) = is_authenticated(&req, &pool) {
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

#[post("/guest/new")]
async fn create_guest(
    pool: web::Data<r2d2::Pool<SqliteConnectionManager>>,
    req: HttpRequest,
) -> impl Responder {
    // Check authentication
    if let Some(author_id) = is_authenticated(&req, &pool) {
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

    println!("Starting Party Hub server on http://127.0.0.1:{}", port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(serve_static)
            .service(create_party)
            .service(update_party)
            .service(delete_party)
            .service(get_parties)
            .service(get_party_details)
            .service(get_guests)
            .service(create_guest)
            .service(get_guest_details)
            .service(update_guest)
            .service(delete_guest)
            .service(add_guest_to_party)
            .service(remove_guest_from_party)
            .service(promote_guest_to_organizer)
            .service(demote_organizer_to_guest)
            .service(details)
            .service(save_answers)
            .service(auth)
            .service(auth_post)
            .service(manage)
            .service(invitation_page)
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
