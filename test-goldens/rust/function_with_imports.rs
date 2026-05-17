use std::collections::HashMap;

use serde::Serialize;

pub fn create_map() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("key".to_string(), "value".to_string());
    map
}

#[derive(Serialize)]
pub struct Config {
    pub name: String,
}
