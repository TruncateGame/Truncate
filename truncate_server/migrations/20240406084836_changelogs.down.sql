-- Add down migration script here
DROP TABLE IF EXISTS changelogs;

ALTER TABLE players
    DROP COLUMN last_known_changelog;
