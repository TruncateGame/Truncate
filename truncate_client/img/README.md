To edit tiles:

- Build Aseprite >=1.3 from source (need tilemap support)
  - When in Aseprite, make sure to never reorder tiles in the palette
    or all IDs in `tex.rs` will need to be remapped.
- Export a horizontal strip tileset to `truncate.png`
- Run `npm i && npm start` in this directory to add the requisite padding
- Update `tex.rs` in `truncate_client` to reference the tile IDs from Aseprite
