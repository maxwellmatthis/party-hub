-- Migration script to update database schema
-- 1. Update guests table: add structured fields (salutation, first, last, email, note)
-- 2. Update parties table: add party attributes (date, respond_until, frozen, public, max_guests)

-- GUESTS TABLE MIGRATION
-- Add new columns to guests table
ALTER TABLE guests ADD COLUMN salutation TEXT DEFAULT '';
ALTER TABLE guests ADD COLUMN first TEXT DEFAULT '';
ALTER TABLE guests ADD COLUMN last TEXT DEFAULT '';
ALTER TABLE guests ADD COLUMN email TEXT DEFAULT '';
ALTER TABLE guests ADD COLUMN note TEXT DEFAULT '';

-- Copy existing name data to first column
UPDATE guests SET first = name;

-- Drop the old name column
ALTER TABLE guests DROP COLUMN name;

-- PARTIES TABLE MIGRATION
-- Add new columns to parties table
ALTER TABLE parties ADD COLUMN date TEXT NOT NULL DEFAULT '';
ALTER TABLE parties ADD COLUMN respond_until TEXT NOT NULL DEFAULT '';
ALTER TABLE parties ADD COLUMN frozen BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE parties ADD COLUMN public BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE parties ADD COLUMN max_guests INTEGER NOT NULL DEFAULT 0;
ALTER TABLE parties ADD COLUMN has_rsvp_block BOOLEAN NOT NULL DEFAULT FALSE;
