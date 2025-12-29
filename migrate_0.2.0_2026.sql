-- Migration script to update database schema
-- 1. Update guests table: add structured fields (salutation, first, last, email, note)
-- 2. Update parties table: add party attributes (name, date, respond_until, frozen, public, max_guests, etc.)

-- GUESTS TABLE MIGRATION
-- Add new columns to guests table
ALTER TABLE guests ADD COLUMN salutation TEXT DEFAULT '';
ALTER TABLE guests ADD COLUMN first TEXT DEFAULT '';
ALTER TABLE guests ADD COLUMN last TEXT DEFAULT '';
ALTER TABLE guests ADD COLUMN email TEXT DEFAULT '';
ALTER TABLE guests ADD COLUMN note TEXT DEFAULT '';
ALTER TABLE guests ADD COLUMN author TEXT DEFAULT '';
ALTER TABLE guests ADD COLUMN selfcreated BOOLEAN NOT NULL DEFAULT FALSE;

-- Copy existing name data to first column
UPDATE guests SET first = name;

-- Try to infer author from invitations for existing guests
-- This links each guest to the author of a party they're invited to
UPDATE guests SET author = (
    SELECT p.author 
    FROM invitations i 
    JOIN parties p ON i.party_id = p.id 
    WHERE i.guest_id = guests.id 
    LIMIT 1
) WHERE author = '';

-- Drop the old name column
ALTER TABLE guests DROP COLUMN name;

-- PARTIES TABLE MIGRATION
-- Add new columns to parties table
ALTER TABLE parties ADD COLUMN name TEXT NOT NULL DEFAULT 'Unnamed Party';
ALTER TABLE parties ADD COLUMN date TEXT NOT NULL DEFAULT '';
ALTER TABLE parties ADD COLUMN respond_until TEXT NOT NULL DEFAULT '';
ALTER TABLE parties ADD COLUMN frozen BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE parties ADD COLUMN public BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE parties ADD COLUMN max_guests INTEGER NOT NULL DEFAULT 0;
ALTER TABLE parties ADD COLUMN has_rsvp_block BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE parties ADD COLUMN duration REAL NOT NULL DEFAULT 0;
ALTER TABLE parties ADD COLUMN location TEXT NOT NULL DEFAULT '';

-- INVITATIONS TABLE MIGRATION
-- Add organizer column
ALTER TABLE invitations ADD COLUMN organizer BOOLEAN NOT NULL DEFAULT FALSE;

-- Drop the old status column (no longer used)
ALTER TABLE invitations DROP COLUMN status;
