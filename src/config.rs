use crate::args::Args;
use clap::Parser;
use log::debug;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct Host {
    pub listen_addr: String,
    pub listen_port: usize,
    pub root_folder: PathBuf,
    pub not_fount_page: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub log_level: String,
    pub log_path: Option<PathBuf>,
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
        let config = fs::read_to_string(config_path).expect("failed to read config file.");
        let mut config: Config = serde_json::from_str(&config).expect("failed to parse config file.");

        // Set config default value.
        if config.log_path.is_none() {
            config.log_path = Some(PathBuf::from("./logs"));
        }

        debug!("{config:?}");
        config
    }
}
