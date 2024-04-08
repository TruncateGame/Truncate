-- Changelogs Table
CREATE TABLE changelogs (
    changelog_id VARCHAR(255) PRIMARY KEY,
    changelog_timestamp TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE players
    ADD COLUMN last_known_changelog TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP;