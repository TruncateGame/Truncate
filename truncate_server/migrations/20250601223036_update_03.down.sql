-- Add down migration script here
DELETE FROM changelogs
WHERE changelog_id = 'update_03';
