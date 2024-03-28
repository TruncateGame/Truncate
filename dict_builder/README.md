# Truncate Dictionary Utilities

This crate builds the dictionaries, frequency lists and more that power Truncate.

Build dict with `cargo run --release`.

## Requirements

The following files must be created:
- `dict_builder/support_data/en_word_freqs.txt`
  - Sourced from https://github.com/hermitdave/FrequencyWords/blob/master/content/2018/en/en_full.txt
- `dict_builder/support_data/wordnik_wordlist.txt`
  - Sourced from https://github.com/wordnik/wordlist
- `dict_builder/support_data/objectionable.json`
  - Generated from the `word_definitions` folder of this repo
- `dict_builder/support_data/generated_scowl_wordlists/*`
  - Containing the files from the `final` directory of the latest release of http://wordlist.aspell.net/

## Adding or removing words

Create a `tranche_<num>_<op>.txt` file inside `support_data` with the words to add or remove.

Reference your new file from `load_additions()` or `load_removals()` in `main.rs`

## Wordlist format

The current Truncate dictionary can be seen inside `final_wordlist.txt`. Excerpt:

```
a 10656 1.0000
aah 10656 0.9993
aalii 10656 0.0000
```

Leftmost is the valid word, lowercase.
In the middle is a heuristic score of how extensible this word is, used by the NPC for evaluating gameplay.
Rightmost is a word frequency from 0 → 1, used to filter what words the NPC "knows" when playing.

This file is compiled into the client, hence the lack of definitions, which must be sourced from the server database.
