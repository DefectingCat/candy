pub struct Config {
    log_level: String,
}

impl Config {
    pub fn new() -> Self {
        Self {
            log_level: "info".to_string(),
        }
    }
}
