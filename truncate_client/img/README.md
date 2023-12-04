To edit tiles:

- Download Aseprite >= 1.3
  - When in Aseprite, make sure to never reorder tiles in the palette
    or all IDs in `tex.rs` will need to be remapped.
- Export tileset to `truncate_packed.png`
  - Make sure to check `Extrude` to add tile padding.
  - Use the `by rows` export, with a constraint on # of columns (max ~50)
