use dashmap::DashMap;
use reqwest::Client;
use std::env;

// TODO: Provide a way for the player to request a definition for a specific word
pub struct Definitions {
    words: DashMap<String, String>,
    client: Client,
}

impl Definitions {
    pub fn new() -> Self {
        Self {
            words: DashMap::new(),
            client: Client::new(),
        }
    }

    pub async fn get_word<S: AsRef<str>>(&self, word: S) -> Option<String> {
        if let Some(def) = self.words.get(word.as_ref()) {
            Some(def.to_owned())
        } else {
            let token = match env::var("WNAPI") {
                Ok(token) => token,
                Err(e) => {
                    println!("No api token: {e}");
                    return None;
                }
            };

            let req = self.client.get(format!(
                "https://api.wordnik.com/v4/word.json/{}/definitions?limit=200&includeRelated=false&sourceDictionaries=all&useCanonical=false&includeTags=false&api_key={token}",
                word.as_ref()
            )).header("Accept", "application/json").send().await;
            println!("{req:#?}");
            let req = req.ok()?;

            let defs = req.json::<serde_json::Value>().await;
            println!("{defs:#?}");
            let defs = defs.ok()?;

            // let definitions = self.client.get(format!(
            //     "https://api.wordnik.com/v4/word.json/{}/definitions?limit=200&includeRelated=false&sourceDictionaries=all&useCanonical=false&includeTags=false&api_key={token}",
            //     word.as_ref()
            // )).header("Accept", "application/json").send().await.ok()?.json::<serde_json::Value>().await.ok()?;

            println!("{defs:#?}");

            None
        }
    }
}
