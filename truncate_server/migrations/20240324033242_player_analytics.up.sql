-- Second Migration: Add new columns to the players table
ALTER TABLE players
    ADD COLUMN last_screen_width INT,
    ADD COLUMN last_screen_height INT,
    ADD COLUMN last_browser_name TEXT,
    ADD COLUMN last_browser_version TEXT,
    ADD COLUMN first_referrer TEXT;