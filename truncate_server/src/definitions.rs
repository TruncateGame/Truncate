use rusqlite::Connection;
use truncate_core::reporting::WordMeaning;

pub struct WordDB {
    pub conn: Connection,
}

impl WordDB {
    pub fn get_word(&mut self, word: &str) -> Option<Vec<WordMeaning>> {
        let mut stmt = self
            .conn
            .prepare("SELECT definitions FROM words WHERE word = ?")
            .unwrap();

        let def_str: Option<String> = stmt
            .query(&[word])
            .unwrap()
            .next()
            .unwrap()
            .map(|row| row.get_unwrap("definitions"));

        def_str
            .map(|def: String| serde_json::from_str(&def).ok())
            .flatten()
    }
}

pub fn read_defs() -> WordDB {
    println!("Loading word definitions...");

    let defs_file = option_env!("TR_DEFS_FILE").unwrap_or_else(|| "/truncate/defs.db");
    let conn = Connection::open(defs_file).unwrap();

    WordDB { conn }
}
