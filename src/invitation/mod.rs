use crate::db::Invitation;
use crate::detect_language;
use actix_web::{HttpResponse, Responder, Scope, get, post, web};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::Deserialize;
use serde_json::json;
use std::fs;

#[get("/{invitation_id}")]
pub async fn invitation_page(
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

#[get("/{invitation_id}")]
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

    // Get guest information for personalization
    let (guest_salutation, guest_first, guest_last) = match conn
        .prepare("SELECT salutation, first, last FROM guests WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&invitation.guest_id], |row| {
                let salutation: String = row.get(0)?;
                let first: String = row.get(1)?;
                let last: String = row.get(2)?;
                Ok((salutation, first, last))
            })
        }) {
        Ok(info) => info,
        Err(_) => return HttpResponse::InternalServerError().body("Guest not found"),
    };

    // Create full name for backward compatibility
    let guest_name = format!("{} {}", guest_first, guest_last).trim().to_string();

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
    let all_other_answers = match conn.prepare("SELECT i.invitation_block_answers, g.first, g.last FROM invitations i JOIN guests g ON i.guest_id = g.id WHERE i.party_id = ?1 AND i.id != ?2 AND i.invitation_block_answers != ''")
        .and_then(|mut stmt| {
            let answer_iter = stmt.query_map([&invitation.party_id, &invitation_id], |row| {
                let answers: String = row.get(0)?;
                let first: String = row.get(1)?;
                let last: String = row.get(2)?;
                let guest_name = format!("{} {}", first, last).trim().to_string();
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
        "guest_salutation": guest_salutation,
        "guest_first": guest_first,
        "guest_last": guest_last,
        "is_organizer": invitation.organizer,
    });

    HttpResponse::Ok()
        .content_type("application/json")
        .body(response.to_string())
}

#[derive(Deserialize)]
struct SaveAnswersRequest {
    answers: serde_json::Value,
}

#[post("/{invitation_id}")]
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
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "error": "Invitation not found"
            }));
        }
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
            let blocks_json =
                serde_json::from_str::<serde_json::Value>(&invitation_blocks).unwrap_or(json!([]));

            let mut valid_ids = std::collections::HashSet::new();
            if let Some(blocks_array) = blocks_json.as_array() {
                for block in blocks_array.iter() {
                    if let Some(block_id) = block.get("id").and_then(|v| v.as_str()) {
                        valid_ids.insert(block_id.to_string());
                    }
                }
            }
            valid_ids
        }
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "Failed to get party data"
            }));
        }
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

pub fn subroutes() -> Scope {
    web::scope("/invitation")
        .service(details)
        .service(save_answers)
}
