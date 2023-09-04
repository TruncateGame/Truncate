const fs = require("fs");
const path = require("path");

const unsummed_file = path.join(__dirname, "unsummed_wordlist.txt");
const unmarked_file = path.join(__dirname, "unmarked_wordlist.txt");

if (!fs.existsSync(unsummed_file)) {
    console.error(`run 'npm run build-freqs' file first to create unsummed_wordlist.txt.`);
    process.exit(1);
}

const wordmap = {};
const word_scores = fs.readFileSync(unsummed_file, { encoding: "utf8" }).split('\n');

for (const line of word_scores) {
    const [word, score, freq] = line.split(' ');
    wordmap[word] = {
        summed_score: parseInt(score),
        base_score: parseInt(score),
        word_freq: freq,
    }
}

let done = 0;
for (const [word, deets] of Object.entries(wordmap)) {
    for (let i = 1; i < word.length; i++) {
        const prefix = word.substring(0, i);
        if (wordmap[prefix]) {
            deets.summed_score += wordmap[prefix].base_score;
        }

        const suffix = word.substring(i);
        if (wordmap[suffix]) {
            deets.summed_score += wordmap[suffix].base_score;
        }
    }

    done += 1;
    if (done % 100 === 0) {
        console.log(`Done ${((done / word_scores.length) * 100).toFixed(2)}%`);
    }
}

let output = Object.entries(wordmap).map(([word, deets]) => {
    return `${word} ${deets.summed_score} ${deets.word_freq}`;
}).join('\n');

fs.writeFileSync(unmarked_file, output);
