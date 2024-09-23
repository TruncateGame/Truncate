-- New player<>changelog table
CREATE TABLE viewed_changelogs (
    player_id UUID REFERENCES players(player_id),
    changelog_id VARCHAR(255) REFERENCES changelogs(changelog_id),
    read_timestamp TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(player_id, changelog_id)
);

-- Mark changelogs as read by relevant players
INSERT INTO viewed_changelogs (player_id, changelog_id, read_timestamp)
SELECT
    p.player_id,
    c.changelog_id,
    p.last_known_changelog
FROM
    players p
JOIN
    changelogs c ON c.changelog_timestamp <= p.last_known_changelog;

-- Intentionally retaining "last_known_changelog" player column for now
