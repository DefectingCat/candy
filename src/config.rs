use std::fs;
use std::path::PathBuf;

use clap::Parser;
use log::debug;
use serde::{Deserialize, Serialize};

use crate::args::Args;

#[derive(Serialize, Deserialize, Debug)]
pub struct Host {
    pub listen_addr: String,
    pub listen_port: usize,
    pub root_folder: PathBuf,
    pub not_found_page: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub log_level: String,
    pub log_path: Option<PathBuf>,
    // Thread number.
    pub works: Option<usize>,
    // pub hosts: Vec<Host>
    pub host: Host,
}

impl Config {
    pub fn new() -> Self {
        let args = Args::parse();
        let config_path = if let Some(path) = args.config {
            path
        } else {
            PathBuf::from("config.json")
        };
        let config = fs::read_to_string(config_path).expect("Failed to read config file.");
        let mut config: Config =
            serde_json::from_str(&config).expect("Failed to parse config file.");

        // Set config default value.
        if config.log_path.is_none() {
            config.log_path = Some(PathBuf::from("./logs"));
        }
        // Set thread numbers to 0 to use all CPU threads.
        if config.works.is_none() {
            config.works = Some(0);
        }

        debug!("{config:?}");
        config
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    #[test]
    fn create_config() {
        let config = Config::new();
        dbg!(&config);
    }

    #[test]
    fn default_workers() {
        let config = Config::new();
        assert_eq!(0, config.works.unwrap());
    }

    #[test]
    fn default_log_path() {
        let config = Config::new();
        assert_eq!("./logs", config.log_path.unwrap().to_string_lossy());
    }
}
