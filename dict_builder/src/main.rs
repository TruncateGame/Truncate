use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, read_dir, File},
    io::{self, BufRead},
    ops::AddAssign,
    path::PathBuf,
};

use dashmap::DashMap;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator,
    IntoParallelRefMutIterator, ParallelIterator,
};

/// This appears to be the best threashold to optimize for valid wordplay while exluding short, "invalid" words.
const MAX_SCOWL_SIZE: usize = 70;

type SourceSize = usize;

/// Primary determiner for which lists do and do not qualify for inclusion in Truncate's validity dictionary.
fn should_include_file(name: impl AsRef<str>) -> (bool, SourceSize) {
    // Currently, special files do not contain any extra Truncate words we desire.
    if name.as_ref().starts_with("special") {
        return (false, 0);
    }

    let (category, rest) = name
        .as_ref()
        .split_once('-')
        .expect("SCOWL files are correctly named");
    let (sub_category, size) = rest
        .split_once('.')
        .expect("SCOWL files are correctly named");

    let size: usize = size.parse().expect("Scowl files are correctly named");
    if size > MAX_SCOWL_SIZE {
        return (false, size);
    }

    // Early exclusion for various classes of word that will never be a valid Truncate word
    match sub_category {
        "words" => { /* allowed, continue */ }
        "abbreviations" => return (false, size),
        "contractions" => return (false, size),
        "proper-names" => return (false, size),
        "upper" => return (false, size),
        other => panic!("Unknown SCOWL sub-category {other}"),
    }

    // Main filtering for spelling categories and their variants.
    (
        match category {
            "american" => true,
            "american_variant_1" => true,
            "american_variant_2" => true,
            "australian" => true,
            "australian_variant_1" => true,
            "australian_variant_2" => true,
            "british" => true,
            "british_variant_1" => true,
            "british_variant_2" => true,
            "british_z" => true,
            "british_z_variant_1" => true,
            "british_z_variant_2" => true,
            "canadian" => true,
            "canadian_variant_1" => true,
            "canadian_variant_2" => true,
            "english" => true,
            "variant_1" => true,
            "variant_2" => true,
            "variant_3" => true,
            _ => panic!("Unknown SCOWL category {category}"),
        },
        size,
    )
}

/// Primary determiner for which words do and do not qualify for inclusion in Truncate's validity dictionary.
fn should_include_word(word: &String, source_size: usize) -> bool {
    // One-letter words in Truncate can be a surprise, exclude them.
    if word.len() < 2 {
        return false;
    }
    // Truncate is ASCII-only — this also helps cut out proper names and words with punctuation
    if !word.chars().all(|c| c.is_ascii_lowercase()) {
        return false;
    }
    // Super short words that are more obscure make Truncate less approachable (ex: xu, ai, ki)
    if word.len() < 3 && source_size > 60 {
        return false;
    }
    return true;
}

fn load_word_frequencies() -> BTreeMap<String, f32> {
    println!("Loading word frequencies from file");
    let frequency_file = File::open(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("support_data/en_word_freqs.txt"),
    )
    .expect("support_data/en_word_freqs.txt file should exist");
    let frequency_lines = io::BufReader::new(frequency_file)
        .lines()
        .flatten()
        .collect::<Vec<_>>();

    let mut frequency_lookup: BTreeMap<String, f32> = BTreeMap::new();

    // Word frequencies are listed in order,
    // so we can just use enumerate() for the rankings
    let mut frequencies = frequency_lines
        .into_par_iter()
        .enumerate()
        .map(|(i, wf)| {
            let (word, _) = wf
                .split_once(' ')
                .expect("Word frequencies are well formed");
            (word.to_string(), i as f32)
        })
        .collect::<Vec<_>>();

    let total_words = frequencies.len() as f32;
    frequencies.par_iter_mut().for_each(|(_, v)| {
        *v = (total_words - *v) / total_words;
    });

    frequency_lookup.extend(frequencies);

    println!("Recalculating word frequency counts");

    frequency_lookup
}

fn load_wordnik_set() -> BTreeSet<String> {
    println!("Loading wordnik data from file");
    let wordnik_file = File::open(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("support_data/wordnik_wordlist.txt"),
    )
    .expect("support_data/wordnik_wordlist.txt file should exist");

    BTreeSet::from_iter(io::BufReader::new(wordnik_file).lines().flatten())
}

fn load_additions() -> BTreeSet<String> {
    println!("Loading additional data from files");

    let files = [
        "support_data/tranche_1_add.txt",
        "support_data/tranche_2_add.txt",
        "support_data/tranche_3_add.txt",
    ]
    .map(|f| {
        File::open(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(f))
            .expect("add files should exist")
    });

    BTreeSet::from_iter(
        files
            .iter()
            .flat_map(|f| io::BufReader::new(f).lines().flatten()),
    )
}

fn load_removals() -> BTreeSet<String> {
    println!("Loading removal data from files");

    let files = ["support_data/tranche_3_del.txt"].map(|f| {
        File::open(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(f))
            .expect("del files should exist")
    });

    BTreeSet::from_iter(
        files
            .iter()
            .flat_map(|f| io::BufReader::new(f).lines().flatten()),
    )
}

fn load_objectionable() -> Vec<String> {
    let input =
        fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("support_data/objectionable.json"))
            .expect("support_data/objectionable.json should exist");
    serde_json::from_slice(&input[..]).expect("objectionable.json should be the expected JSON")
}

fn score_extension(target: &String, larger_word: &String) -> Option<usize> {
    if larger_word <= target {
        return None;
    }
    if larger_word.starts_with(target) || larger_word.ends_with(target) {
        let diff = larger_word.len() - target.len();
        if diff >= 5 {
            return Some(1);
        } else {
            return Some((5 - diff).pow(2));
        }
    }
    None
}

fn main() {
    println!("Starting the dict builder");
    let frequency_lookup = load_word_frequencies();

    println!("Loading candidate wordlists");
    let files = read_dir(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("support_data/generated_scowl_wordlists/"),
    )
    .expect("support_data/generated_scowl_wordlists directory should exist");

    let mut scowl_word_list: BTreeSet<String> = BTreeSet::new();

    for file in files.flatten() {
        let (included, source_size) = should_include_file(&file.file_name().to_string_lossy());

        if included {
            println!("Processing {:?} into the word set", file.file_name());

            let spelling_list = File::open(file.path()).unwrap();
            let spelling_lines = io::BufReader::new(spelling_list).lines().flatten();

            scowl_word_list.extend(spelling_lines.filter(|w| should_include_word(w, source_size)));
        } else {
            println!(">> Skipping {:?}", file.file_name());
        }
    }

    // To help filter out less desired words from SCOWL, we require words to _also_ be in the Wordnik games set.
    let wordnik_word_list = load_wordnik_set();
    let mut final_wordlist: BTreeSet<_> =
        wordnik_word_list.intersection(&scowl_word_list).collect();

    let additions = load_additions();
    final_wordlist.extend(additions.iter());

    let removals = load_removals();
    for removal in removals {
        final_wordlist.remove(&removal);
    }

    println!("{} words in the total set.", final_wordlist.len());
    println!("Calculating word substring counts");

    struct WordData {
        substring_score: usize,
        frequency: f32,
        objectionable: bool,
    }

    let backprop_points: DashMap<&String, usize> = DashMap::new();
    let objectionable = load_objectionable();

    let mut scored_word_list = final_wordlist
        .par_iter()
        .map(|word| {
            let frequency = frequency_lookup.get(*word).cloned().unwrap_or(0.0);
            let links: Vec<_> = final_wordlist
                .iter()
                .filter_map(|w| score_extension(*word, *w).map(|score| (w, score)))
                .collect();
            let substring_score: usize = links.iter().map(|(_, score)| score).sum();

            for (word, _) in links.into_iter() {
                _ = backprop_points
                    .entry(*word)
                    .or_default()
                    .add_assign(substring_score);
            }

            (
                *word,
                WordData {
                    substring_score,
                    frequency,
                    objectionable: objectionable.contains(word),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    println!("Backpropagating word substring scores");
    scored_word_list.iter_mut().for_each(|(word, data)| {
        if let Some(pts) = backprop_points.get(word) {
            data.substring_score += *pts;
        }
    });

    println!("Formatting the output file");
    let word_list = scored_word_list
        .into_iter()
        .map(
            |(
                word,
                WordData {
                    substring_score,
                    frequency,
                    objectionable,
                },
            )| {
                format!(
                    "{}{word} {substring_score} {frequency:.4}",
                    if objectionable { "*" } else { "" }
                )
            },
        )
        .collect::<Vec<_>>();

    println!("Writing output file");

    let output_file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("final_wordlist.txt");
    let output_file_contents = word_list.join("\n");

    fs::write(output_file_path, output_file_contents).expect("Output file should be writable");
}
