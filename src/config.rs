use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub repo: String,
}

pub fn read_config(path: &PathBuf) -> Config {
    let s = fs::read_to_string(path).expect("Unable to read config file");
    serde_json::from_str(s.as_str()).expect("Failed to evaluate content of config file")
}
