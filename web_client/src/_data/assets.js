const fs = require('fs');
const path = require('path');
const subsetFont = require('subset-font');

const load_tiles = () => {
    const tile_asset = path.join(__dirname, "../../../truncate_client/img/truncate_processed.png");
    const tile_data = fs.readFileSync(tile_asset);
    const tile_string = tile_data.toString('base64');
    return `data:image/png;base64,${tile_string}`;
}

const load_font = async () => {
    const font_asset = path.join(__dirname, "../../../truncate_client/font/PressStart2P-Regular.ttf");
    const font_data = fs.readFileSync(font_asset);
    const ascii_chars = Array.from({ length: 95 }, (_, i) => String.fromCharCode(i + 32)).join('');
    const subset = await subsetFont(font_data, ascii_chars, {
        targetFormat: 'woff2',
    });
    const font_string = subset.toString('base64');
    return `data:font/woff2;base64,${font_string}`;
}

module.exports = async function () {
    const font = await load_font();
    const tiles = load_tiles();

    return { font, tiles }
}