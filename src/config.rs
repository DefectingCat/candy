use crate::args::Args;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub log_level: String,
}

impl Config {
    pub fn new() -> Self {
        let args = Args::parse();
        let config_path = if let Some(path) = args.config {
            path
        } else {
            panic!("cannot access config file!")
        };
        let config = fs::read_to_string(config_path).expect("failed to read config file.");
        let config: Config = serde_json::from_str(&config).expect("");
        dbg!(&config);

        Self {
            log_level: "info".to_string(),
        }
    }
}
