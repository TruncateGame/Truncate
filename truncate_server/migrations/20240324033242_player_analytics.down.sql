-- Add down migration script here
ALTER TABLE players
    DROP COLUMN last_screen_width,
    DROP COLUMN last_screen_height,
    DROP COLUMN last_browser_name,
    DROP COLUMN last_browser_version,
    DROP COLUMN first_referrer;
