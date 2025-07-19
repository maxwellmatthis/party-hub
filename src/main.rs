use actix_web::{App, HttpResponse, HttpServer, Responder, get, post, web};
use r2d2::Pool;
use serde_json::json;
use std::fs;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate rusqlite;
use r2d2_sqlite::SqliteConnectionManager;
use serde::Deserialize;

mod db;
use db::prepare_db;

// Function to detect preferred language from Accept-Language header
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

#[get("/main.js")]
async fn script() -> impl Responder {
    let html_content = fs::read_to_string("static/main.js")
        .unwrap_or_else(|_| "<h1>404: File Not Found</h1>".to_string());
    HttpResponse::Ok()
        .content_type("application/javascript")
        .body(html_content)
}

#[get("/style.css")]
async fn style() -> impl Responder {
    let html_content = fs::read_to_string("static/style.css")
        .unwrap_or_else(|_| "<h1>404: File Not Found</h1>".to_string());
    HttpResponse::Ok()
        .content_type("text/css")
        .body(html_content)
}

#[get("/{invitation_id}")]
async fn index(req: actix_web::HttpRequest) -> impl Responder {
    // Detect language from Accept-Language header
    let language = detect_language(&req);
    
    let filename = match language.as_str() {
        "de" => "static/index_de.html",
        _ => "static/index_en.html", // Default to English
    };
    
    let html_content = fs::read_to_string(filename)
        .unwrap_or_else(|_| "<h1>404: File Not Found</h1>".to_string());
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

    let (party_id, guest_id) = match conn
        .prepare("SELECT party_id, guest_id FROM invitations WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&invitation_id], |row| {
                let party_id: String = row.get(0)?;
                let guest_id: String = row.get(1)?;
                Ok((party_id, guest_id))
            })
        }) {
        Ok((party_id, guest_id)) => (party_id, guest_id),
        Err(_) => return HttpResponse::BadRequest().body("Invitation not found"),
    };

    // Get guest name
    let guest_name = match conn
        .prepare("SELECT name FROM guests WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&guest_id], |row| {
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
            stmt.query_row([&party_id], |row| {
                let invitation_blocks: String = row.get(0)?;
                Ok(invitation_blocks)
            })
        }) {
        Ok(invitation_blocks) => invitation_blocks,
        Err(_) => return HttpResponse::InternalServerError().body("Party data not found"),
    };

    let invitation_block_answers = match conn
        .prepare("SELECT invitation_block_answers FROM invitations WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&invitation_id], |row| {
                let answers: String = row.get(0)?;
                Ok(answers)
            })
        }) {
        Ok(answers) => answers,
        Err(_) => return HttpResponse::InternalServerError().body("Invitation answers not found"),
    };

    // Get all other guests' answers for the same party (excluding current invitation)
    let all_other_answers = match conn.prepare("SELECT invitation_block_answers FROM invitations WHERE party_id = ?1 AND id != ?2 AND invitation_block_answers != ''")
        .and_then(|mut stmt| {
            let answer_iter = stmt.query_map([&party_id, &invitation_id], |row| {
                let answers: String = row.get(0)?;
                Ok(answers)
            })?;

            let mut all_answers = Vec::new();
            for answer_result in answer_iter {
                if let Ok(answer_str) = answer_result {
                    if let Ok(answer_json) = serde_json::from_str::<serde_json::Value>(&answer_str) {
                        all_answers.push(answer_json);
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
    let mut public_block_indices = std::collections::HashSet::new();

    if let Some(blocks_array) = blocks_json.as_array() {
        for (i, block) in blocks_array.iter().enumerate() {
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
                        public_block_indices.insert(i.to_string());
                    }
                }
            }
        }
    }

    // Filter other guests' answers to only include public blocks
    let filtered_other_answers: Vec<serde_json::Value> = all_other_answers
        .into_iter()
        .map(|guest_answers| {
            let mut filtered_guest = serde_json::Map::new();
            if let Some(answers_obj) = guest_answers.as_object() {
                for (block_index, answer) in answers_obj {
                    if public_block_indices.contains(block_index) {
                        filtered_guest.insert(block_index.clone(), answer.clone());
                    }
                }
            }
            serde_json::Value::Object(filtered_guest)
        })
        .collect();

    let response = json!({
        "invitation_blocks": serde_json::from_str::<serde_json::Value>(&invitation_blocks).unwrap_or(json!([])),
        "invitation_block_answers": serde_json::from_str::<serde_json::Value>(&invitation_block_answers).unwrap_or(json!([])),
        "other_guests_answers": filtered_other_answers,
        "guest_name": guest_name,
    });

    HttpResponse::Ok()
        .content_type("application/json")
        .body(response.to_string())
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
        return HttpResponse::BadRequest().json(json!({
            "error": "Invitation not found"
        }));
    }

    // Convert answers to JSON string
    let answers_json = answers.to_string();

    // Update the invitation with new answers using prepared statement
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    prepare_db().expect("FATAL: Unable to set up DB!");
    let manager = SqliteConnectionManager::file("party.db");
    let pool = r2d2::Pool::new(manager).unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(script)
            .service(style)
            .service(details)
            .service(index)
            .service(save_answers)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
