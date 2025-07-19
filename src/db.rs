use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct InvitationBlock {
    pub template: String,
    pub content: String,
}

#[derive(Debug)]
pub struct Party {
    id: String,
    invitation_blocks: Vec<InvitationBlock>,
}

pub struct Invitation {
    id: String,
    guest_id: String,
    party_id: String,
    invitation_block_answers: HashMap<u32, String>,
}

#[derive(Debug)]
pub struct Guest {
    id: String,
    name: String,
}

pub fn prepare_db() -> Result<()> {
    let conn = Connection::open("./party.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS parties (
            id    TEXT PRIMARY KEY,
            invitation_blocks JSON
        )",
        (),
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS invitations (
            id    TEXT PRIMARY KEY,
            guest_id TEXT NOT NULL,
            party_id TEXT NOT NULL,
            status TEXT NOT NULL,
            invitation_block_answers JSON
        )",
        (),
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS guests (
            id    TEXT PRIMARY KEY,
            name  TEXT NOT NULL
        )",
        (),
    )?;

    Ok(())
}
