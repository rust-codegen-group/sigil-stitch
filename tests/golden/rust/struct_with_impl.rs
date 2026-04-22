use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Application configuration.
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub values: HashMap<String, i64>,
}

impl Config {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), values: HashMap::new() }
    }
}
