#!/usr/bin/env node

const fs = require("fs");
const path = require("path");

const INPUT_FILE = path.join(__dirname, "../truncate_client/img/tile_order");
const OUTPUT_RUST = path.join(__dirname, "../truncate_client/src/utils/tex/tiles.rs");
const OUTPUT_JS = path.join(__dirname, "../web_client/src/_data/tiles.js");

const raw_tiles = fs
    .readFileSync(INPUT_FILE, { encoding: "utf8" })
    .split("\n");

const quad_tiles = {};
for (const tile of raw_tiles) {
    if (/_(nw|ne|se|sw)$/i.test(tile)) {
        const tile_base = tile.replace(/_(nw|ne|se|sw)$/i, '');
        const tile_corner = tile.match(/_(nw|ne|se|sw)$/i)?.[1];
        quad_tiles[tile_base] = quad_tiles[tile_base] || {};
        quad_tiles[tile_base][tile_corner?.toLowerCase() || "??"] = true
    }
}

const rs_map = [
    `#![cfg_attr(rustfmt, rustfmt_skip)]\n`,
    `#![allow(dead_code)]\n`,
    `use super::{Tex, TexQuad};\n`,
    `const fn t(tile: usize) -> Tex {`,
    `    Tex { tile, tint: None }`,
    `}\n`,
    `pub const MAX_TILE: usize = ${raw_tiles.length};`,
    ...raw_tiles.map((tile, i) =>
        `pub const ${tile}: Tex = t(${i});`
    ),
    `\npub mod quad {`,
    `    use super::*;\n`,
    ...Object.entries(quad_tiles).map(([tile, quadrants]) =>
        `    pub const ${tile}: TexQuad = [${[
            quadrants.nw ? `${tile}_NW` : `NONE`,
            quadrants.ne ? `${tile}_NE` : `NONE`,
            quadrants.se ? `${tile}_SE` : `NONE`,
            quadrants.sw ? `${tile}_SW` : `NONE`,
        ].join(', ')}];`
    ),
    `}`
].join("\n");

fs.writeFileSync(OUTPUT_RUST, rs_map);

const js_map = [
    `const tiles = {`,
    ...raw_tiles.map((tile, i) =>
        `    ${tile}: ${i},`
    ),
    `};\n`,
    `const quads = {`,
    ...Object.entries(quad_tiles).map(([tile, quadrants]) =>
        `    ${tile}: [[${[
            quadrants.nw ? `tiles.${tile}_NW` : `tiles.NONE`,
            quadrants.ne ? `tiles.${tile}_NE` : `tiles.NONE`,
        ].join(', ')}],[${[
            quadrants.sw ? `tiles.${tile}_SW` : `tiles.NONE`,
            quadrants.se ? `tiles.${tile}_SE` : `tiles.NONE`,
        ].join(', ')}]],`
    ),
    `};\n`,
    `module.exports = async function () {`,
    `    return {`,
    `        ...tiles,`,
    `        QUAD: {`,
    `            ...quads,`,
    `        }`,
    `    }`,
    `}`,
].join("\n");

fs.writeFileSync(OUTPUT_JS, js_map);
