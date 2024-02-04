# Truncate Dictionary Utilities

This crate builds the dictionaries, frequency lists and more that power Truncate.

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
