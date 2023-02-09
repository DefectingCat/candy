#[derive(Debug)]
pub struct Config {
    pub log_level: String,
}

impl Config {
    pub fn new() -> Self {
        Self {
            log_level: "info".to_string(),
        }
    }
}
