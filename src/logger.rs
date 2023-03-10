use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use chrono::Local;
use env_logger::{Builder, Env};

use crate::config::Config;

pub fn create_file(file_path: &PathBuf) -> Result<()> {
    if file_path.exists() {
        return Ok(());
    } else {
        fs::create_dir_all(
            file_path
                .parent()
                .expect("Can not access log parent folder"),
        )
        .expect("Can not create log folder");
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

    let now = Local::now();
    let formatted = format!("{}.log", now.format("%Y-%m-%d"));
    let file_path = PathBuf::from(&log_path).join(formatted);
    create_file(&file_path)?;

    let log_level = config
        .lock()
        .expect("Can not get config file.")
        .log_level
        .clone();

    let env = Env::default().filter_or("RUA_LOG_LEVEL", &log_level);
    let mut builder = Builder::from_env(env);

    builder
        .format(move |buf, record| {
            let formatted = format!("{}", now.format("%Y-%m-%d %H:%M:%S"));
            let log = format!("{} - {} - {}", formatted, record.level(), record.args());

            let mut target = OpenOptions::new()
                .write(true)
                .read(true)
                .create(true)
                .append(true)
                .open(&file_path)
                .expect("");
            writeln!(target, "{log}").expect("Can not write log to file.");
            writeln!(buf, "{log}")
        })
        // .target(env_logger::Target::Pipe(target))
        // .filter(None, LevelFilter::Info)
        .init();

    Ok(())
}
