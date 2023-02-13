use crate::handles::handle_connection;
use crate::thread_pool::ThreadPool;
use config::Config;
use env_logger::Env;
use log::info;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

mod args;
mod config;
mod handles;
mod logger;
mod thread_pool;

fn main() {
    let config = Arc::new(Mutex::new(Config::new()));
    let log_level = config
        .lock()
        .expect("Can not get config file.")
        .log_level
        .clone();

    logger::init_logger(Arc::clone(&config)).unwrap();

    let env = Env::default().filter_or("RUA_LOG_LEVEL", &log_level);
    env_logger::init_from_env(env);
    info!("Server starting.");

    let thread_pool = ThreadPool::new(0);

    let (addr, port) = {
        let host = &config.lock().expect("Can not get config file.").host;
        (host.listen_addr.clone(), host.listen_port)
    };

    let listener = TcpListener::bind(format!("{addr}:{port}"))
        .unwrap_or_else(|_| panic!("Can not listen on {addr}:{port}"));
    info!("Listen on {addr}:{port}.");

    for stream in listener.incoming() {
        let config = Arc::clone(&config);
        let stream = stream.unwrap();
        let job = Box::new(move || {
            handle_connection(&stream, config);
        });
        thread_pool.execute(job);
    }
}
