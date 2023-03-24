use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use anyhow::Result;

use log::{error, info};

use config::Config;
use tokio::net::TcpListener;

use crate::handles::handle_connection;

mod args;
mod config;
mod consts;
mod error;
mod frame;
mod handles;
mod logger;
mod thread_pool;

#[tokio::main]
async fn main() -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = Arc::clone(&running);
    ctrlc::set_handler(move || r.store(false, Ordering::SeqCst)).expect("");

    let config = Arc::new(Mutex::new(Config::new()));
    if let Err(err) = logger::init_logger(Arc::clone(&config)) {
        error!("Failed to create logger; {}", err.to_string());
        exit(1);
    }
    info!("Server starting.");

    let (addr, port) = {
        let host = &config.lock().expect("Can not get config file.").host;
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
    while running.load(Ordering::SeqCst) {
        let (mut stream, _) = listener.accept().await?;
        let config  = Arc::clone(&confg);

        tokio::task::spawn(async move {
            handle_connection(&stream, config);
        });
    }
    println!("Exiting...");


    Ok(())
}
