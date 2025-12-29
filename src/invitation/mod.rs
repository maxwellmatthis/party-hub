use crate::db::Invitation;
use crate::detect_language;
use actix_web::{HttpResponse, Responder, Scope, get, post, web};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::Deserialize;
use serde_json::json;
use std::fs;

// Helper function to format date and time based on language
// Expects ISO datetime format: YYYY-MM-DD or YYYY-MM-DDTHH:MM or YYYY-MM-DDTHH:MM:SS
fn format_date_time(date_str: &str, language: &str) -> (String, String) {
    if date_str.is_empty() {
        return (String::new(), String::new());
    }

    // Try to parse as full datetime first (with seconds)
    if let Ok(datetime) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S") {
        let formatted_date = match language {
            "de" => datetime.format("%d.%m.%Y").to_string(), // DD.MM.YYYY
            _ => datetime.format("%m/%d/%Y").to_string(),    // MM/DD/YYYY (English)
        };
        let formatted_time = match language {
            "de" => datetime.format("%H:%M").to_string(), // 24-hour
            _ => datetime.format("%I:%M %p").to_string(), // 12-hour with AM/PM
        };
        return (formatted_date, formatted_time);
    }

    // Try to parse as datetime without seconds (datetime-local format)
    if let Ok(datetime) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M") {
        let formatted_date = match language {
            "de" => datetime.format("%d.%m.%Y").to_string(),
            _ => datetime.format("%m/%d/%Y").to_string(),
        };
        let formatted_time = match language {
            "de" => datetime.format("%H:%M").to_string(),
            _ => datetime.format("%I:%M %p").to_string(),
        };
        return (formatted_date, formatted_time);
    }

    // Fallback to date-only format
    if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        let formatted_date = match language {
            "de" => date.format("%d.%m.%Y").to_string(),
            _ => date.format("%m/%d/%Y").to_string(),
        };
        return (formatted_date, String::new());
    }

    (String::new(), String::new())
}

#[get("/{invitation_id}")]
pub async fn invitation_page(
    path: web::Path<String>,
    req: actix_web::HttpRequest,
    db: web::Data<Pool<SqliteConnectionManager>>,
) -> impl Responder {
    let id = path.into_inner();

    // Verify this is actually a valid invitation ID or public party ID
    let conn = match db.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("Database connection failed"),
    };

    // First check if it's a public party ID
    let is_public_party = conn
        .prepare("SELECT public FROM parties WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&id], |row| {
                let is_public: bool = row.get(0)?;
                Ok(is_public)
            })
        })
        .unwrap_or(false);

    if is_public_party {
        // This is a public party - serve the invitation page for anonymous viewing
        let language = detect_language(&req);
        let filename = match language.as_str() {
            "de" => "pages/de/invitation_de.html",
            _ => "pages/en/invitation_en.html",
        };

        let html_content = fs::read_to_string(filename)
            .unwrap_or_else(|_| "<h1>404: File Not Found</h1>".to_string());
        return HttpResponse::Ok()
            .content_type("text/html")
            .body(html_content);
    }

    // Check if invitation exists
    let invitation_exists = conn
        .prepare("SELECT COUNT(*) FROM invitations WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&id], |row| {
                let count: i32 = row.get(0)?;
                Ok(count > 0)
            })
        })
        .unwrap_or(false);

    if !invitation_exists {
        let language = detect_language(&req);
        let filename = match language.as_str() {
            "de" => "pages/de/not_found_de.html",
            _ => "pages/en/not_found_en.html",
        };
        let html_content =
            fs::read_to_string(filename).unwrap_or_else(|_| "<h1>404: Not Found</h1>".to_string());
        return HttpResponse::NotFound()
            .content_type("text/html")
            .body(html_content);
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
    req: actix_web::HttpRequest,
    db: web::Data<Pool<SqliteConnectionManager>>,
) -> impl Responder {
    let id = path.into_inner();

    let conn = match db.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("Database connection failed"),
    };

    // Check if this is a public party first
    let public_party_check = conn
        .prepare("SELECT id, invitation_blocks, date, name, author FROM parties WHERE id = ?1 AND public = 1")
        .and_then(|mut stmt| {
            stmt.query_row([&id], |row| {
                let party_id: String = row.get(0)?;
                let invitation_blocks: String = row.get(1)?;
                let date: String = row.get(2)?;
                let party_name: String = row.get(3)?;
                let author_id: String = row.get(4)?;
                Ok((party_id, invitation_blocks, date, party_name, author_id))
            })
        });

    if let Ok((party_id, invitation_blocks, party_date, party_name, author_id)) = public_party_check
    {
        // This is a public party - return anonymous guest data
        let language = detect_language(&req);
        let (formatted_date, formatted_time) = format_date_time(&party_date, &language);

        // Get author name separately
        let author_name = conn
            .prepare("SELECT name FROM authors WHERE id = ?1")
            .and_then(|mut stmt| {
                stmt.query_row([&author_id], |row| {
                    let name: String = row.get(0)?;
                    Ok(name)
                })
            })
            .unwrap_or_else(|_| "Unknown".to_string());

        let response = json!({
            "invitation_blocks": serde_json::from_str::<serde_json::Value>(&invitation_blocks).unwrap_or(json!([])),
            "invitation_block_answers": json!({}),
            "other_guests_answers": json!([]),
            "guest_name": "Anonymous",
            "guest_salutation": "",
            "guest_first": "Anonymous",
            "guest_last": "",
            "party_name": party_name,
            "party_date": formatted_date,
            "party_time": formatted_time,
            "author_name": author_name,
            "is_organizer": false,
            "is_public_view": true,
            "party_id": party_id,
        });

        return HttpResponse::Ok()
            .content_type("application/json")
            .body(response.to_string());
    }

    // Not a public party, treat as regular invitation
    let invitation = match conn
        .prepare("SELECT id, guest_id, party_id, invitation_block_answers, organizer FROM invitations WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&id], Invitation::from_row)
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

    // Get invitation blocks, party date, party name, and has_rsvp_block flag
    let (invitation_blocks, party_date, party_name, author_id, has_rsvp_block) = match conn
        .prepare("SELECT invitation_blocks, date, name, author, has_rsvp_block FROM parties WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&invitation.party_id], |row| {
                let invitation_blocks: String = row.get(0)?;
                let date: String = row.get(1)?;
                let party_name: String = row.get(2)?;
                let author_id: String = row.get(3)?;
                let has_rsvp_block: bool = row.get(4)?;
                Ok((invitation_blocks, date, party_name, author_id, has_rsvp_block))
            })
        }) {
        Ok(data) => data,
        Err(_) => return HttpResponse::InternalServerError().body("Party data not found"),
    };

    // Get author name separately
    let author_name = conn
        .prepare("SELECT name FROM authors WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&author_id], |row| {
                let name: String = row.get(0)?;
                Ok(name)
            })
        })
        .unwrap_or_else(|_| "Unknown".to_string());

    // Format date and time based on language
    let language = detect_language(&req);
    let (formatted_date, formatted_time) = format_date_time(&party_date, &language);

    // Get all other guests' answers for the same party (excluding current invitation)
    // Include guest names for organizer view
    let all_other_answers = match conn.prepare("SELECT i.invitation_block_answers, g.first, g.last FROM invitations i JOIN guests g ON i.guest_id = g.id WHERE i.party_id = ?1 AND i.id != ?2 AND i.invitation_block_answers != ''")
        .and_then(|mut stmt| {
            let answer_iter = stmt.query_map([&invitation.party_id, &id], |row| {
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

    // Parse invitation blocks to determine which are public and find attendance block
    let blocks_json =
        serde_json::from_str::<serde_json::Value>(&invitation_blocks).unwrap_or(json!([]));
    let mut public_block_ids = std::collections::HashSet::new();
    let mut attendance_block_id: Option<String> = None;

    if let Some(blocks_array) = blocks_json.as_array() {
        for block in blocks_array.iter() {
            // Get the block ID
            if let Some(block_id) = block.get("id").and_then(|v| v.as_str()) {
                // Check if this is an attendance block
                if let Some(template) = block.get("template").and_then(|v| v.as_str()) {
                    if template == "attendance" {
                        attendance_block_id = Some(block_id.to_string());
                    }
                }

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
        // Mark answers from guests who haven't RSVP'd yes with "(?)" (only if has_rsvp_block is true)
        all_other_answers
            .into_iter()
            .map(|(guest_answers, guest_name)| {
                // Only check RSVP status if the party has an RSVP block
                let has_rsvped_yes = if has_rsvp_block {
                    if let Some(attendance_id) = &attendance_block_id {
                        guest_answers
                            .get(attendance_id)
                            .and_then(|v| v.as_i64())
                            .map(|answer| answer == 0)
                            .unwrap_or(false)
                    } else {
                        true // No attendance block found, so don't mark
                    }
                } else {
                    true // No RSVP block in party, so don't apply RSVP filtering
                };

                let mut answer_with_names = serde_json::Map::new();
                if let Some(answers_obj) = guest_answers.as_object() {
                    for (block_id, answer) in answers_obj {
                        // Create answer object with guest name
                        // Add (?) if not RSVP'd yes, but not for the attendance block itself
                        let is_attendance_block =
                            Some(block_id.as_str()) == attendance_block_id.as_deref();
                        let display_name =
                            if has_rsvped_yes || is_attendance_block || !has_rsvp_block {
                                guest_name.clone()
                            } else {
                                format!("{} (?)", guest_name)
                            };
                        let answer_with_name = json!({
                            "answer": answer,
                            "guest_name": display_name
                        });
                        answer_with_names.insert(block_id.clone(), answer_with_name);
                    }
                }
                serde_json::Value::Object(answer_with_names)
            })
            .collect()
    } else {
        // Regular guests can only see public answers with guest names
        // Filter out answers from guests who haven't RSVP'd yes (only if has_rsvp_block is true)
        // Exception: the attendance block itself should always show all responses if public
        all_other_answers
            .into_iter()
            .filter_map(|(guest_answers, guest_name)| {
                // Only check RSVP status if the party has an RSVP block
                let has_rsvped_yes = if has_rsvp_block {
                    if let Some(attendance_id) = &attendance_block_id {
                        guest_answers
                            .get(attendance_id)
                            .and_then(|v| v.as_i64())
                            .map(|answer| answer == 0)
                            .unwrap_or(false)
                    } else {
                        true // No attendance block found, so include all answers
                    }
                } else {
                    true // No RSVP block in party, so don't apply RSVP filtering
                };

                let mut filtered_guest = serde_json::Map::new();
                if let Some(answers_obj) = guest_answers.as_object() {
                    for (block_id, answer) in answers_obj {
                        if public_block_ids.contains(block_id) {
                            // For the attendance block itself, always show all responses
                            // For other blocks, only show if guest RSVP'd yes
                            let is_attendance_block =
                                Some(block_id.as_str()) == attendance_block_id.as_deref();

                            if is_attendance_block || !has_rsvp_block || has_rsvped_yes {
                                // Create answer object with guest name for public answers
                                let answer_with_name = json!({
                                    "answer": answer,
                                    "guest_name": guest_name
                                });
                                filtered_guest.insert(block_id.clone(), answer_with_name);
                            }
                        }
                    }
                }

                // Only include this guest if they have at least one visible answer
                if filtered_guest.is_empty() {
                    None
                } else {
                    Some(serde_json::Value::Object(filtered_guest))
                }
            })
            .collect()
    };

    let response = json!({
        "invitation_blocks": serde_json::from_str::<serde_json::Value>(&invitation_blocks).unwrap_or(json!([])),
        "invitation_block_answers": invitation.get_answers_json(),
        "other_guests_answers": filtered_other_answers,
        "guest_id": invitation.guest_id,
        "guest_name": guest_name,
        "guest_salutation": guest_salutation,
        "guest_first": guest_first,
        "guest_last": guest_last,
        "party_name": party_name,
        "party_date": formatted_date,
        "party_time": formatted_time,
        "author_name": author_name,
        "is_organizer": invitation.organizer,
        "is_public_view": false,
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
    req: actix_web::HttpRequest,
) -> impl Responder {
    let id = path.into_inner();
    let answers = &json.answers;
    let language = detect_language(&req);

    let conn = match db.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("Database connection failed"),
    };

    // Check if this is a public party
    let public_party_check = conn
        .prepare("SELECT id FROM parties WHERE id = ?1 AND public = 1")
        .and_then(|mut stmt| {
            stmt.query_row([&id], |row| {
                let party_id: String = row.get(0)?;
                Ok(party_id)
            })
        });

    if public_party_check.is_ok() {
        // This is a public party - user needs to create guest first
        // Return special response indicating they need to register
        return HttpResponse::Ok().json(json!({
            "status": "registration_required",
            "message": "Please complete registration"
        }));
    }

    // Get invitation and party information
    let party_id = match conn
        .prepare("SELECT party_id FROM invitations WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&id], |row| {
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

    // Check if party is frozen or deadline has passed
    let (frozen, respond_until) = match conn
        .prepare("SELECT frozen, respond_until FROM parties WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&party_id], |row| {
                let frozen: bool = row.get(0)?;
                let respond_until: String = row.get(1)?;
                Ok((frozen, respond_until))
            })
        }) {
        Ok(data) => data,
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "Failed to get party status"
            }));
        }
    };

    // Check if party is frozen
    if frozen {
        let error_msg = match language.as_str() {
            "de" => "Diese Party ist eingefroren und akzeptiert keine Antworten mehr",
            _ => "This party is frozen and no longer accepting responses",
        };
        return HttpResponse::Forbidden().json(json!({
            "error": error_msg
        }));
    }

    // Check if respond_until deadline has passed
    if !respond_until.is_empty() {
        let now = chrono::Local::now().naive_local();

        // Try to parse as datetime first (with or without seconds)
        let deadline_passed = if let Ok(deadline) =
            chrono::NaiveDateTime::parse_from_str(&respond_until, "%Y-%m-%dT%H:%M:%S")
        {
            now > deadline
        } else if let Ok(deadline) =
            chrono::NaiveDateTime::parse_from_str(&respond_until, "%Y-%m-%dT%H:%M")
        {
            now > deadline
        } else if let Ok(deadline_date) =
            chrono::NaiveDate::parse_from_str(&respond_until, "%Y-%m-%d")
        {
            // If only date, consider deadline as end of day
            let deadline = deadline_date
                .and_hms_opt(23, 59, 59)
                .unwrap_or(deadline_date.and_hms_opt(0, 0, 0).unwrap());
            now > deadline
        } else {
            false
        };

        if deadline_passed {
            let error_msg = match language.as_str() {
                "de" => "Die Frist zum Antworten auf diese Einladung ist abgelaufen",
                _ => "The deadline for responding to this invitation has passed",
            };
            return HttpResponse::Forbidden().json(json!({
                "error": error_msg
            }));
        }
    }

    // Get valid block IDs from the party's invitation blocks
    let (valid_block_ids, attendance_block_id, max_guests) = match conn
        .prepare("SELECT invitation_blocks, max_guests FROM parties WHERE id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&party_id], |row| {
                let invitation_blocks: String = row.get(0)?;
                let max_guests: i64 = row.get(1)?;
                Ok((invitation_blocks, max_guests))
            })
        }) {
        Ok((invitation_blocks, max_guests)) => {
            // Parse invitation blocks to extract valid block IDs and attendance block ID
            let blocks_json =
                serde_json::from_str::<serde_json::Value>(&invitation_blocks).unwrap_or(json!([]));

            let mut valid_ids = std::collections::HashSet::new();
            let mut attendance_id: Option<String> = None;
            if let Some(blocks_array) = blocks_json.as_array() {
                for block in blocks_array.iter() {
                    if let Some(block_id) = block.get("id").and_then(|v| v.as_str()) {
                        valid_ids.insert(block_id.to_string());
                        // Check if this is an attendance block
                        if block.get("template").and_then(|v| v.as_str()) == Some("attendance") {
                            attendance_id = Some(block_id.to_string());
                        }
                    }
                }
            }
            (valid_ids, attendance_id, max_guests)
        }
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "Failed to get party data"
            }));
        }
    };

    // Check if user is trying to RSVP "yes" (answer = 0) and max_guests limit is reached
    // Note: "no" (2) and "maybe" (1) responses are always allowed to let people free up space
    // Only enforce this if there's an attendance block AND max_guests is set
    if max_guests > 0 && attendance_block_id.is_some() {
        let attendance_id = attendance_block_id.as_ref().unwrap();
        if let Some(new_answer) = answers.get(attendance_id) {
            if new_answer.as_i64() == Some(0) {
                // User is trying to RSVP yes - check if they currently have "yes"
                // If they already have yes, allow them to keep it or update other answers
                let current_attendance_answer = conn
                    .prepare("SELECT invitation_block_answers FROM invitations WHERE id = ?1")
                    .and_then(|mut stmt| {
                        stmt.query_row([&id], |row| {
                            let current_answers: String = row.get(0)?;
                            Ok(current_answers)
                        })
                    })
                    .ok()
                    .and_then(|ans_str| {
                        if ans_str.is_empty() {
                            None
                        } else {
                            serde_json::from_str::<serde_json::Value>(&ans_str).ok()
                        }
                    })
                    .and_then(|ans_json| ans_json.get(attendance_id).cloned())
                    .and_then(|ans| ans.as_i64());

                // Only check limit if they're NOT currently "yes" (changing from no/maybe/unanswered to yes)
                let is_changing_to_yes = current_attendance_answer != Some(0);

                if is_changing_to_yes {
                    // Count current yes responses from OTHER invitations
                    let yes_count = conn
                        .prepare("SELECT invitation_block_answers FROM invitations WHERE party_id = ?1 AND id != ?2")
                        .and_then(|mut stmt| {
                            let answer_iter = stmt.query_map([&party_id, &id], |row| {
                                let answers: String = row.get(0)?;
                                Ok(answers)
                            })?;

                            let mut count = 0i64;
                            for answer_result in answer_iter {
                                if let Ok(answer_str) = answer_result {
                                    // Skip empty answer strings
                                    if answer_str.is_empty() {
                                        continue;
                                    }
                                    if let Ok(answer_json) = serde_json::from_str::<serde_json::Value>(&answer_str) {
                                        if let Some(att_answer) = answer_json.get(attendance_id) {
                                            if let Some(answer_val) = att_answer.as_i64() {
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
                        .unwrap_or(0);

                    if yes_count >= max_guests {
                        let error_msg = match language.as_str() {
                            "de" => "Diese Party hat die maximale Anzahl an Gästen erreicht",
                            _ => "This party has reached its maximum number of guests",
                        };
                        return HttpResponse::Forbidden().json(json!({
                            "error": error_msg
                        }));
                    }
                }
            }
        }
    }

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
        .and_then(|mut stmt| stmt.execute([&answers_json, &id]));

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

#[get("/register")]
pub async fn register(req: actix_web::HttpRequest) -> impl Responder {
    let language = detect_language(&req);
    let filename = match language.as_str() {
        "de" => "pages/de/public_guest_de.html",
        _ => "pages/en/public_guest_en.html",
    };

    let html_content =
        fs::read_to_string(filename).unwrap_or_else(|_| "<h1>404: File Not Found</h1>".to_string());
    HttpResponse::Ok()
        .content_type("text/html")
        .body(html_content)
}

#[get("/{invitation_id}/ics")]
async fn download_calendar(
    path: web::Path<String>,
    db: web::Data<Pool<SqliteConnectionManager>>,
) -> impl Responder {
    let invitation_id = path.into_inner();

    let conn = match db.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("Database connection failed"),
    };

    // Get party details for this invitation
    let party_info = match conn
        .prepare("SELECT p.name, p.date, p.duration, p.location FROM parties p JOIN invitations i ON p.id = i.party_id WHERE i.id = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([&invitation_id], |row| {
                let name: String = row.get(0)?;
                let date: String = row.get(1)?;
                let duration: f64 = row.get(2)?;
                let location: String = row.get(3)?;
                Ok((name, date, duration, location))
            })
        }) {
        Ok(info) => info,
        Err(_) => return HttpResponse::NotFound().body("Invitation not found"),
    };

    let (party_name, party_date, duration_hours, location) = party_info;

    // Parse the party date
    let start_dt = if let Ok(dt) =
        chrono::NaiveDateTime::parse_from_str(&party_date, "%Y-%m-%dT%H:%M:%S")
    {
        dt
    } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&party_date, "%Y-%m-%dT%H:%M") {
        dt
    } else {
        return HttpResponse::BadRequest().body("Invalid party date format");
    };

    // Calculate end time (convert hours to minutes)
    let duration_minutes = (duration_hours * 60.0).round() as i64;
    let end_dt = start_dt + chrono::Duration::minutes(duration_minutes);

    // Generate unique ID for the event
    let event_uid = format!("{}@party-hub", invitation_id);

    // Get current timestamp for DTSTAMP
    let now = chrono::Utc::now();

    // Format dates for iCalendar (UTC format)
    let dtstart = start_dt.format("%Y%m%dT%H%M%S").to_string();
    let dtend = end_dt.format("%Y%m%dT%H%M%S").to_string();
    let dtstamp = now.format("%Y%m%dT%H%M%SZ").to_string();

    // Build invitation URL
    let invitation_url = format!("https://party-hub.com/{}", invitation_id); // TODO: Use actual host

    // Generate iCalendar file
    let ics_content = format!(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Party Hub//EN\r\nCALSCALE:GREGORIAN\r\nMETHOD:PUBLISH\r\nBEGIN:VEVENT\r\nUID:{}\r\nDTSTAMP:{}\r\nDTSTART:{}\r\nDTEND:{}\r\nSUMMARY:{}\r\nLOCATION:{}\r\nDESCRIPTION:Invitation link: {}\r\nURL:{}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
        event_uid,
        dtstamp,
        dtstart,
        dtend,
        party_name.replace(",", "\\,").replace("\n", "\\n"),
        location.replace(",", "\\,").replace("\n", "\\n"),
        invitation_url,
        invitation_url
    );

    HttpResponse::Ok()
        .content_type("text/calendar; charset=utf-8")
        .append_header((
            "Content-Disposition",
            format!(
                "inline; filename=\"{}.ics\"",
                party_name.replace("/", "-").replace("\\", "-")
            ),
        ))
        .body(ics_content)
}

pub fn subroutes() -> Scope {
    web::scope("/invitation")
        .service(details)
        .service(save_answers)
        .service(download_calendar)
}
