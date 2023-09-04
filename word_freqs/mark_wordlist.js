const fs = require("fs");
const path = require("path");

const unmarked_file = path.join(__dirname, "unmarked_wordlist.txt");
const marking_file = path.join(__dirname, "../word_definitions/objectionable.json");
const final_file = path.join(__dirname, "final_wordlist.txt");

if (!fs.existsSync(unmarked_file)) {
    console.error(`run 'npm run build-sums' file first to create unmarked_wordlist.txt.`);
    process.exit(1);
}
if (!fs.existsSync(marking_file)) {
    console.error(`run the build script in '../word_definitions' first to create objectionable.json`);
    process.exit(1);
}

const words = fs.readFileSync(unmarked_file, { encoding: "utf8" }).split('\n');
const marks = JSON.parse(fs.readFileSync(marking_file, { encoding: "utf8" }));

for (let i = 0; i < words.length; i++) {
    let is_objectionable = marks.includes(words[i].split(' ')[0]);

    if (is_objectionable) {
        words[i] = `*${words[i]}`;
    }
}

fs.writeFileSync(final_file, words.join('\n'));
