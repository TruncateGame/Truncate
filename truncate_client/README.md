# Truncate Client

This crate represents all frontend code for rendering and playing Truncate.

It supports building as a native application, or as a WebAssembly module to load into a browser.
In both cases, `egui` (via `eframe`) is the GUI library used for all rendering and inputs.

## General client info

`egui` is an immediate mode UI framework, which means the entire screen will be repainted every frame.

Truncate targets a 4fps tick when idle, so all idle animations (e.g. wind effects) should be made for this tick rate.

On input (e.g. mouse movement) or during animations (e.g. tutorial text, battle animation), Truncate will run at a significantly higher framerate.

## Editing Tutorials

Tutorials can be found in the `tutorials/*.yml` files, whose format should be self-explanatory. Changes here are automatically compiled into the client.

## Editing images

See the repo root CONTRIBUTING.md for steps.

## Code structure

Generally, `app_inner.rs` is the main place to start (skipping the egui-specific logic in `app_outer`, `main`, and `lib`).

Communications go through the `*_comms.rs` files, but all handling is done inside `app_inner.rs`.

Large regions of gameplay, e.g. single player vs multiplayer, existing in the `regions/` directory.
Within this directory, `active_game.rs` is a special case, as it tends to be used by every other region to render the actual gameplay UI.

Smaller items exist inside `lil_bits/`, e.g. board rendering or dialogs.

Both of the above directories are in need of some love re: code structure and organization!
