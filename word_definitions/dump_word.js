/**
 * Run `node dump_word.js some_word` to print out the matching
 * word blobs from the raw data source.
 */

const path = require('path');
const fs = require('fs');
const readline = require('readline');
const jsonlines = require('jsonlines');

const input_file = path.join(__dirname, "kaikki.org-dictionary-English.json");

if (!fs.existsSync(input_file)) {
    console.error(`Need to build word definitions from a dictionary reference.`);
    console.error(`Download the English JSON data from https://kaikki.org/dictionary/English/index.html`);
    console.error(`And place the file at ${input_file}`);
    process.exit(1);
}

const target_word = process.argv[2];
console.log(`Looking for ${target_word}`);

const rl = readline.createInterface({
    input: fs.createReadStream(input_file),
    crlfDelay: Infinity
});
const parser = jsonlines.parse();

const writeWord = (word_json) => {
    if (word_json.word.toLowerCase() == target_word) {
        console.log(JSON.stringify(word_json, null, 2));
    }
}

parser.on('data', function (data) {
    writeWord(data);
});

rl.on('line', (line) => {
    parser.write(line);
    parser.write(`\n`);
});

rl.on('close', () => {
    parser.end();
});
