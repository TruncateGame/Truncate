#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const { Jimp } = require("jimp");

async function processImage() {
  const IMAGE_INPUT_PATH = path.join(
    __dirname,
    "../truncate_client/img/truncate_packed_pre.png",
  );
  const IMAGE_OUTPUT_PATH = path.join(
    __dirname,
    "../truncate_client/img/truncate_packed.png",
  );

  const image = await Jimp.read(IMAGE_INPUT_PATH);

  image.scan((x, y, idx) => {
    const red = image.bitmap.data[idx + 0];
    const green = image.bitmap.data[idx + 1];
    const blue = image.bitmap.data[idx + 2];

    // Find pure magenta and make it transparent
    if (red === 255 && green === 0 && blue === 255) {
      image.bitmap.data[idx + 3] = 0;
    }
  });

  await image.write(IMAGE_OUTPUT_PATH);
}

processImage().catch((err) => {
  console.error("Error processing image:", err);
});

const INPUT_FILE = path.join(__dirname, "../truncate_client/img/tile_order");
const OUTPUT_RUST = path.join(
  __dirname,
  "../truncate_client/src/utils/tex/tiles.rs",
);
const OUTPUT_JS = path.join(__dirname, "../web_client/src/_data/tiles.js");

const raw_tiles = fs
  .readFileSync(INPUT_FILE, { encoding: "utf8" })
  .split("\n")
  .filter((t) => /\w/.test(t));

const quad_tiles = {};
for (const tile of raw_tiles) {
  if (/_(nw|ne|se|sw)$/i.test(tile)) {
    const tile_base = tile.replace(/_(nw|ne|se|sw)$/i, "");
    const tile_corner = tile.match(/_(nw|ne|se|sw)$/i)?.[1];
    quad_tiles[tile_base] = quad_tiles[tile_base] || {};
    quad_tiles[tile_base][tile_corner?.toLowerCase() || "??"] = true;
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
  ...raw_tiles.map((tile, i) => `pub const ${tile}: Tex = t(${i});`),
  `\npub mod quad {`,
  `    use super::*;\n`,
  ...Object.entries(quad_tiles).map(
    ([tile, quadrants]) =>
      `    pub const ${tile}: TexQuad = [${[
        quadrants.nw ? `${tile}_NW` : `NONE`,
        quadrants.ne ? `${tile}_NE` : `NONE`,
        quadrants.se ? `${tile}_SE` : `NONE`,
        quadrants.sw ? `${tile}_SW` : `NONE`,
      ].join(", ")}];`,
  ),
  `}`,
].join("\n");

fs.writeFileSync(OUTPUT_RUST, rs_map);

const js_map = [
  `const tiles = {`,
  ...raw_tiles.map((tile, i) => `    ${tile}: ${i},`),
  `};\n`,
  `const quads = {`,
  ...Object.entries(quad_tiles).map(
    ([tile, quadrants]) =>
      `    ${tile}: [[${[
        quadrants.nw ? `tiles.${tile}_NW` : `tiles.NONE`,
        quadrants.ne ? `tiles.${tile}_NE` : `tiles.NONE`,
      ].join(", ")}],[${[
        quadrants.sw ? `tiles.${tile}_SW` : `tiles.NONE`,
        quadrants.se ? `tiles.${tile}_SE` : `tiles.NONE`,
      ].join(", ")}]],`,
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
