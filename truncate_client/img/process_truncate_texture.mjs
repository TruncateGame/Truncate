import Jimp from "jimp";

const img = await Jimp.read("truncate.png");
const { width, height } = img.bitmap;

const tile_size = 16;
if (height !== tile_size || width % tile_size !== 0) {
    throw new Error('Input image must be 16px high and a multiple of 16px wide.');
}

const num_tiles = width / tile_size;
const num_columns = num_tiles * 2;

const output_img = await new Jimp(width + num_columns, height + 2);

const is_col_to_dupe = x => x % tile_size === 0 || x % tile_size === 15;

// Duplicate the column pixels between sprites
let skip_x = 0;
for (let x = 0; x < width; x += 1) {
    for (let y = 0; y < height; y += 1) {
        const color = img.getPixelColor(x, y);

        let output_x = x + skip_x;
        let output_y = y + 1;

        output_img.setPixelColor(color, output_x, output_y);
        if (is_col_to_dupe(x)) {
            output_img.setPixelColor(color, output_x + 1, output_y);
        }

        // Also duplicate the top and bottom pixels to new header and footer rows
        if (y === 0) {
            output_img.setPixelColor(color, output_x, 0);
            output_img.setPixelColor(color, output_x + 1, 0);
        } else if (y === height - 1) {
            output_img.setPixelColor(color, output_x, output_y + 1);
            output_img.setPixelColor(color, output_x + 1, output_y + 1);
        }
    }
    if (is_col_to_dupe(x)) {
        skip_x += 1;
    }
}

console.log(`New tiles are ${output_img.bitmap.width / num_tiles}px wide with padding`);

await output_img.writeAsync("truncate_processed.png");