# Truncate, Technically

Interested in Truncate under the hood? This file will (try to) get you up to speed.

Truncate is majority developed in Rust, so having a recent version of Rust installed is a prerequisite.
Some supporting scripts are written in NodeJS, which should also be installed.

Running Truncate's server requires a local Postgres database to exist on the default port.

## Main directory overview

- `dict_builder`
  - The crate that generates Truncate's dictionary from a range of input sources.
- `truncate_client`
  - The frontend rendering code for Truncate, implemented in egui.
- `truncate_core`
  - The core engine of Truncate's game logic, and implements the rules and helpers for performing actions in a game.
- `truncate_dueller`
  - A development crate not loaded in production. It simulates and pregenerates the future daily puzzles to ensure they're winnable and fair.
- `truncate_server`
  - The backend that the client uses for things like multiplayer games and account persistence.
- `trunkshipper`
  - A stub used to deploy a log aggregation tool for monitoring.
- `web_client`
  - The static website providing the initial menu and wasm loading code.
- `word_definitions`
  - A node script that generates the SQLite database full of word definitions for lookups. Eventually to be moved into the `dict_builder`

## Getting up and running locally

### Running the server

```bash
cd truncate_server && cargo run
```

Note: This will run by default without any word definitions for lookups. To run with word definitions:
- `gunzip` the `word_definitions/defs.db.gz` file into a `word_definitions/local_defs.db` file.
- Modify the `cargo run` above to `TR_DEFS_FILE=../word_definitions/local_defs.db cargo run`

### Running the web client

Building the WASM web client is done by running `./.backstage/build-web-client.sh` from the root of the repo.

Then, start the 11ty dev server with `cd web_client/src && npm start`.

### Running the native client

Truncate also runs as a native client, though with significantly more rough edges in the menu space.

Make sure your server is running, then execute:
```bash
cd truncate_client && cargo run --release ws://0.0.0.0:8080
```

## Specific details

See the `README.md` file within each directory for more information in that realm.

## Common tasks

### Running tests

From the root of the repo, `cargo insta test --review` will compile and test all crates.
Reminder to have your local Postgres running, as sqlx will require this to compile.

### Generate a new batch of daily puzzles

```bash
cd truncate_dueller && cargo run --release
```

### Generating the tileset

- Using Aseprite, open the `truncate_client/img/truncate.aseprite` file
- After making changes, select `File > Export > Export Tileset`
- Use the following settings:
  - **Layout** tab:
    - **Sheet Type**: By Rows
    - **Constraints**: Fixed # of Columns: 30
    - Uncheck **Merge Duplicates** and **Ignore Empty**
  - **Sprite** tab:
    - **Source**: Tilesets
    - **Layers**: Selected layers (do not **Split Layers**)
    - **Frames**: All Frames (do not **Split Tags**)
  - **Borders** tab:
    - **Border Padding**: 0
    - **Spacing**: 0
    - **Inner Padding**: 0
    - **Trim Sprite**: unchecked
    - **Trim Cels**: unchecked
    - **Extrude**: CHECKED!
  - **Output**:
    - **Output File**: `truncate_packed.png`
    - **JSON Data** unchecked

If you modified the tile order at all:
- Edit `truncate_client/img/tile_order` to match the new ordering of tiles
- If you added something 32x32, and thus taking up four tiles, suffix their tile names with `_NW`, `_NE`, `_SW`, and `_SE`
- From the root of the repo, run `.backstage/build-tile-data.js` to rerun the codegen for using tiles

