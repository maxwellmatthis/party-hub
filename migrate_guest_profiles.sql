-- Migration script to update guests table structure
-- Copies name to first, leaves salutation and last empty
-- Adds email and note fields for guest management

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
