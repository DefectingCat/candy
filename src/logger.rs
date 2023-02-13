use crate::config::Config;
use anyhow::Result;
use std::sync::{Arc, Mutex};

pub fn create_file() {}

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
    dbg!(&log_path);

    Ok(())
}
