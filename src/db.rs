use rusqlite::{Connection, Result, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct InvitationBlock {
    pub template: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Party {
    pub id: String,
    pub name: String,
    pub author: String,
    pub invitation_blocks: String, // Store as JSON string for simplicity
    pub date: String, // ISO 8601 date string
    pub respond_until: String, // ISO 8601 date string
    pub frozen: bool,
    pub public: bool,
    pub max_guests: i64,
    pub has_rsvp_block: bool,
}

impl Party {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Party {
            id: row.get("id")?,
            name: row.get("name")?,
            author: row.get("author")?,
            invitation_blocks: row.get("invitation_blocks")?,
            date: row.get("date")?,
            respond_until: row.get("respond_until")?,
            frozen: row.get("frozen")?,
            public: row.get("public")?,
            max_guests: row.get("max_guests")?,
            has_rsvp_block: row.get("has_rsvp_block")?,
        })
    }

    // Convenience method to parse invitation_blocks as JSON
    pub fn get_invitation_blocks_json(&self) -> serde_json::Value {
        serde_json::from_str(&self.invitation_blocks).unwrap_or(serde_json::json!([]))
    }

    // Convert to JSON representation for API responses
    pub fn to_summary_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "date": self.date,
            "respond_until": self.respond_until,
            "frozen": self.frozen,
            "public": self.public,
            "max_guests": self.max_guests,
            "has_rsvp_block": self.has_rsvp_block
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Invitation {
    pub id: String,
    pub guest_id: String,
    pub party_id: String,
    pub invitation_block_answers: String, // Store as JSON string
    pub organizer: bool,
}

impl Invitation {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Invitation {
            id: row.get("id")?,
            guest_id: row.get("guest_id")?,
            party_id: row.get("party_id")?,
            invitation_block_answers: row.get("invitation_block_answers")?,
            organizer: row.get("organizer")?,
        })
    }

    // Convenience method to parse invitation_block_answers as JSON
    pub fn get_answers_json(&self) -> serde_json::Value {
        serde_json::from_str(&self.invitation_block_answers).unwrap_or(serde_json::json!({}))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Guest {
    pub id: String,
    pub salutation: String,
    pub first: String,
    pub last: String,
    pub email: String,
    pub note: String,
    pub author: String,
}

impl Guest {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Guest {
            id: row.get("id")?,
            salutation: row.get("salutation")?,
            first: row.get("first")?,
            last: row.get("last")?,
            email: row.get("email")?,
            note: row.get("note")?,
            author: row.get("author")?,
        })
    }

    // Convert to JSON representation for API responses
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "salutation": self.salutation,
            "first": self.first,
            "last": self.last,
            "email": self.email,
            "note": self.note
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Author {
    pub id: String,
    pub name: String,
    pub author_secret: String,
}

impl Author {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Author {
            id: row.get("id")?,
            name: row.get("name")?,
            author_secret: row.get("author_secret")?,
        })
    }
}

pub fn prepare_db() -> Result<()> {
    let conn = Connection::open("./party.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS parties (
            id    TEXT PRIMARY KEY,
            name  TEXT NOT NULL,
            author TEXT NOT NULL,
            invitation_blocks JSON,
            date TEXT NOT NULL DEFAULT '',
            respond_until TEXT NOT NULL DEFAULT '',
            frozen BOOLEAN NOT NULL DEFAULT FALSE,
            public BOOLEAN NOT NULL DEFAULT FALSE,
            max_guests INTEGER NOT NULL DEFAULT 0,
            has_rsvp_block BOOLEAN NOT NULL DEFAULT FALSE
        )",
        (),
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS invitations (
            id    TEXT PRIMARY KEY,
            guest_id TEXT NOT NULL,
            party_id TEXT NOT NULL,
            invitation_block_answers JSON,
            organizer BOOLEAN NOT NULL DEFAULT FALSE
        )",
        (),
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS guests (
            id    TEXT PRIMARY KEY,
            salutation TEXT NOT NULL DEFAULT '',
            first TEXT NOT NULL DEFAULT '',
            last TEXT NOT NULL DEFAULT '',
            email TEXT NOT NULL DEFAULT '',
            note TEXT NOT NULL DEFAULT '',
            author TEXT NOT NULL
        )",
        (),
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS authors (
            id    TEXT PRIMARY KEY,
            name  TEXT NOT NULL,
            author_secret TEXT NOT NULL
        )",
        (),
    )?;

    Ok(())
}
