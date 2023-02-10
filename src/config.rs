use crate::args::Args;
use clap::Parser;
use log::debug;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct Host {
    pub listen_addr: String,
    pub root_folder: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub log_level: String,
    // pub hosts: Vec<Host>
    pub host: Host,
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
        let config: Config = serde_json::from_str(&config).expect("failed to parse config file.");
        debug!("{config:?}");
        config
    }
}
