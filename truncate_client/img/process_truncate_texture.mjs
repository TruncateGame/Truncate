import Jimp from "jimp";

const img = await Jimp.read("truncate.png");
const { width, height } = img.bitmap;

const tile_size = 18;
if (height !== tile_size || width % tile_size !== 0) {
    throw new Error(`Input image must be ${tile_size}px high and a multiple of ${tile_size}px wide.`);
}

const num_tiles = width / tile_size;

const num_cols = 50;
const num_rows = Math.floor(num_tiles / num_cols) + 1;

const output_width = num_cols * tile_size;
const output_height = num_rows * tile_size;

const output_img = await new Jimp(output_width, output_height);

for (let x = 0; x < width; x += 1) {
    for (let y = 0; y < height; y += 1) {
        const color = img.getPixelColor(x, y);

        const new_x = x % output_width;
        const new_y = y + (Math.floor(x / output_width) * tile_size);

        output_img.setPixelColor(color, new_x, new_y);
    }
}

console.log(`New image is ${num_cols}x${num_rows} tiles; ${output_width}x${output_height} pixels`);

await output_img.writeAsync("truncate_packed.png");