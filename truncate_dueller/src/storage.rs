use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

fn note_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("seed_notes.yml")
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SeedNote {
    pub rerolls: usize,
    pub best_player: usize,
    pub verification: u64,
}

#[derive(Default, Serialize, Deserialize)]
pub struct NotesFile {
    pub notes: BTreeMap<u32, SeedNote>,
}

pub fn load_file() -> NotesFile {
    let notes = std::fs::read_to_string(note_file())
        .map(|file| {
            serde_yaml::from_str(&file)
                .expect("If the file exists, it should match the notes format")
        })
        .unwrap_or_default();

    notes
}

pub fn write_file(notes: NotesFile) {
    let output_content = serde_yaml::to_string(&notes).unwrap();
    std::fs::write(note_file(), output_content).expect("Writing notes should succeed");
}
