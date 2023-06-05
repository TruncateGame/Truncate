const fs = require("fs");
const path = require("path");

const freq_file = path.join(__dirname, "en_word_freqs.txt");
const word_file = path.join(__dirname, "wordnik_wordlist.txt");
const final_file = path.join(__dirname, "final_wordlist.txt");

if (!fs.existsSync(freq_file)) {
    console.error(`Need to build word frequencies from a frequency reference.`);
    console.error(`Download the English data from https://github.com/hermitdave/FrequencyWords/blob/master/content/2018/en/en_full.txt`);
    console.error(`And place the file at ${freq_file}`);
    process.exit(1);
}

const frequencies = {};

const raw_freqs = fs.readFileSync(freq_file, { encoding: "utf8" });
const freq_entries = raw_freqs.split('\n');
for (let i = 0; i < freq_entries.length; i += 1) {
    const split = freq_entries[i].split(' ');
    if (split.length !== 2) continue;
    const word = split[0];
    frequencies[word] = ((freq_entries.length - i) / freq_entries.length).toFixed(4);
}

const words = fs.readFileSync(word_file, { encoding: "utf8" }).split('\n');
let output = [];

let done = 0;
for (const word of words) {
    let freq = frequencies[word];
    let extension_regex = new RegExp(`^${word}[a-z]+ \\d+$`, 'mg');
    let extensions = raw_freqs.match(extension_regex);

    output.push(`${word} ${extensions?.length || 0} ${frequencies[word] || 0}`)
    done += 1;
    if (done % 100 === 0) {
        console.log(`Done ${done}/${words.length}`);
    }
}

fs.writeFileSync(final_file, output.join('\n'));
