const fs = require("fs");
const path = require("path");

const freq_file = path.join(__dirname, "en_word_freqs.txt");
const word_file = path.join(__dirname, "wordnik_wordlist.txt");
const unsummed_file = path.join(__dirname, "unsummed_wordlist.txt");

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

const all_words = fs.readFileSync(word_file, { encoding: "utf8" });
const words = all_words.split('\n');
let output = [];

let done = 0;
for (const word of words) {
    const single_prefix_regex = new RegExp(`^[a-z]${word}$`, 'mg');
    const single_prefixes = all_words.match(single_prefix_regex)?.length || 0;

    const prefix_regex = new RegExp(`^[a-z]+${word}$`, 'mg');
    const prefixes = all_words.match(prefix_regex)?.length || 0;

    const suffix_regex = new RegExp(`^${word}[a-z]+$`, 'mg');
    const suffixes = all_words.match(suffix_regex)?.length || 0;

    const score = (single_prefixes * 20) + (prefixes * 3) + (suffixes);

    const wordline = `${word} ${score} ${frequencies[word] || 0}`;
    output.push(wordline)
    done += 1;
    if (done % 100 === 0) {
        console.log(wordline);
        console.log(`Done ${((done / words.length) * 100).toFixed(2)}%`);
    }
}

fs.writeFileSync(unsummed_file, output.join('\n'));
