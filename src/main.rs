use std::net::TcpListener;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use log::{error, info};

use config::Config;

use crate::handles::handle_connection;
use crate::thread_pool::ThreadPool;

mod args;
mod config;
mod frame;
mod handles;
mod logger;
mod thread_pool;

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = Arc::clone(&running);
    ctrlc::set_handler(move || r.store(false, Ordering::SeqCst)).expect("");

    let config = Arc::new(Mutex::new(Config::new()));
    if let Err(err) = logger::init_logger(Arc::clone(&config)) {
        error!("Failed to create logger; {}", err.to_string());
        exit(1);
    }
    info!("Server starting.");

    let work_num = config.lock().expect("").works.unwrap();
    let thread_pool = Arc::new(Mutex::new(ThreadPool::new(work_num)));
    let (addr, port) = {
        let host = &config.lock().expect("Can not get config file.").host;
        (host.listen_addr.clone(), host.listen_port)
    };
    let listener = TcpListener::bind(format!("{addr}:{port}")).unwrap_or_else(|err| {
        error!("Can not listen on {addr}:{port}; {}", err.to_string());
        exit(1);
    });

    let pool = Arc::clone(&thread_pool);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let config = Arc::clone(&config);
            let stream = stream.unwrap();
            let job = Box::new(move || {
                handle_connection(&stream, config);
            });
            pool.lock().unwrap().execute(job);
        }
    });

    info!("Listen on {addr}:{port}.");
    while running.load(Ordering::SeqCst) {}
    thread_pool.lock().unwrap().exit();
}
