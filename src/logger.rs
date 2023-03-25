use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use chrono::Local;
use env_logger::{Builder, Env};
use tokio::{
    fs::{self, OpenOptions},
    sync::Mutex,
};

use crate::config::Config;

pub async fn create_file(file_path: &PathBuf) -> Result<()> {
    if file_path.exists() {
        return Ok(());
    } else {
        fs::create_dir_all(
            file_path
                .parent()
                .expect("Can not access log parent folder"),
        )
        .await?;
    }
    Ok(())
}

pub async fn init_logger(config: Arc<Mutex<Config>>) -> Result<()> {
    let log_path = if let Some(path) = config.lock().await.log_path.clone() {
        path
    } else {
        panic!("Can not access log path")
    };

    let now = Local::now();
    let formatted = format!("{}.log", now.format("%Y-%m-%d"));
    let file_path = PathBuf::from(&log_path).join(formatted);
    create_file(&file_path).await?;

    let log_level = config.lock().await.log_level.clone();

    let env = Env::default().filter_or("RUA_LOG_LEVEL", &log_level);
    let mut builder = Builder::from_env(env);

    builder
        .format(move |buf, record| {
            let formatted = format!("{}", now.format("%Y-%m-%d %H:%M:%S"));
            let log = format!("{} - {} - {}", formatted, record.level(), record.args());

            let file_path = file_path.clone();
            tokio::spawn(async move {
                let target = OpenOptions::new()
                    .write(true)
                    .read(true)
                    .create(true)
                    .append(true)
                    .open(&file_path)
                    .await;
            });
            writeln!(buf, "{log}")
        })
        // .target(env_logger::Target::Pipe(target))
        // .filter(None, LevelFilter::Info)
        .init();

    Ok(())
}
