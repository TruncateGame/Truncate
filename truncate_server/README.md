# Truncate Server

The backend that handles:

- Multiplayer lobbies and games, running the actual game logic
- Returning definitions for word lookups in puzzles and single player games
- Persisting daily puzzles in the database for those with a login token

### Making database changes

To create a new migration, run `cd truncate_server && cargo sqlx migrate add <migration name>`.
These migrations will run automatically on server startup. Please also write a `down` migration.

After changing a migration, or changing any queries in the Rust code, run `cd truncate_server && cargo sqlx prepare` so that CI will build.
