use anyhow::Result;
use std::process::exit;
use std::sync::Arc;

use log::{error, info};

use config::Config;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::Mutex;

use crate::handles::handle_connection;

mod args;
mod config;
mod consts;
mod error;
mod frame;
mod handles;
mod logger;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(Mutex::new(Config::new()));
    if let Err(err) = logger::init_logger(Arc::clone(&config)).await {
        error!("Failed to create logger; {}", err.to_string());
        exit(1);
    }
    info!("Server starting.");

    let (addr, port) = {
        let host = &config.lock().await.host;
        let addr = if let Some(addr) = &host.listen_addr {
            addr.clone()
        } else {
            exit(1);
        };
        let port = if let Some(port) = host.listen_port {
            port
        } else {
            exit(1);
        };
        (addr, port)
    };

    let listener = TcpListener::bind(format!("{addr}:{port}")).await?;
    info!("Listen on {addr}:{port}.");

    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let config = Arc::clone(&config);

            tokio::spawn(async move {
                handle_connection(stream, config).await;
            });
        }
    });

    match signal::ctrl_c().await {
        Ok(()) => {
            println!("Exiting...");
            return Ok(());
        }
        Err(err) => {
            error!("Unable to listen for shutdown signal: {}", err);
        }
    }
    // while running.load(Ordering::SeqCst) {
    // }
    println!("Exiting...");
    Ok(())
}
