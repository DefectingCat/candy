use crate::config::Config;
use anyhow::Result;
use chrono::{Datelike, Local};
use log::{error, info};
use std::fs;
use std::fs::File;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub fn create_file(log_path: &PathBuf) -> Result<()> {
    let now = Local::now();
    let filename = format!("{}-{:02}-{:02}.log", now.year(), now.month(), now.day());
    dbg!(&now);
    let file_path = PathBuf::from(&log_path).join(filename);
    if File::open(&file_path).is_ok() {
        return Ok(());
    }
    match fs::read_dir(log_path) {
        Ok(dir) => {
            dbg!(&dir);
            File::create(file_path)?;
        }
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                info!("Log folder not exist; creating");
                fs::create_dir(log_path)?;
                File::create(file_path)?;
            }
            _ => {
                error!("{}", err.to_string());
            }
        },
    }
    Ok(())
}

pub fn init_logger(config: Arc<Mutex<Config>>) -> Result<()> {
    let log_path = if let Some(path) = config
        .lock()
        .expect("Cannot lock config file")
        .log_path
        .clone()
    {
        path
    } else {
        panic!("Can not access log path")
    };
    create_file(&log_path).unwrap();

    Ok(())
}
