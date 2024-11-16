use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone)]
pub struct Tutorial {
    pub effective_day: u32,
    pub rules: Vec<Category>,
    pub splash_message: Option<Vec<String>>,
    pub changelog_name: Option<String>,
    pub priority: Option<ChangePriority>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Category {
    pub category: String,
    pub scenarios: Vec<Scenario>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Scenario {
    pub name: String,
    pub board: String,
    pub player_hand: String,
    pub computer_hand: String,
    pub steps: Vec<ScenarioStep>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ScenarioStep {
    OwnMove {
        you: String,
        gets: char,
        description: String,
    },
    ComputerMove {
        computer: String,
        gets: char,
        description: String,
    },
    Dialog {
        message: String,
    },
    EndAction {
        end_message: String,
    },
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum ChangePriority {
    High,
    Low,
}

pub fn rules(for_day: u32) -> Tutorial {
    [
        serde_yaml::from_slice::<Tutorial>(include_bytes!("../../tutorials/rules_2.yml"))
            .expect("Tutorial should match Tutorial format"),
        serde_yaml::from_slice::<Tutorial>(include_bytes!("../../tutorials/rules_1.yml"))
            .expect("Tutorial should match Tutorial format"),
        serde_yaml::from_slice::<Tutorial>(include_bytes!("../../tutorials/rules_0.yml"))
            .expect("Tutorial should match Tutorial format"),
    ]
    .into_iter()
    .find(|r| r.effective_day <= for_day || r.effective_day == 0)
    .expect("Some ruleset should apply for any given day")
}

pub fn changelogs() -> HashMap<&'static str, Tutorial> {
    HashMap::from([
        (
            "update_01",
            serde_yaml::from_slice(include_bytes!("../../tutorials/update_01.yml"))
                .expect("Tutorial should match Tutorial format"),
        ),
        (
            "update_02",
            serde_yaml::from_slice(include_bytes!("../../tutorials/update_02.yml"))
                .expect("Tutorial should match Tutorial format"),
        ),
    ])
}
