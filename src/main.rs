extern crate core;

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
mod thread_pool;

fn main() {
    let config = Arc::new(Mutex::new(Config::new()));

    let temp = Arc::clone(&config);
    let log_level = &temp.lock().unwrap().log_level;
    let env = Env::default().filter_or("RUA_LOG_LEVEL", log_level);
    env_logger::init_from_env(env);
    info!("server starting.");

    let thread_pool = ThreadPool::new(0);

    let listener = TcpListener::bind("127.0.0.1:4000").expect("cannon listen on port 4000");
    for stream in listener.incoming() {
        let config = Arc::clone(&config);
        let stream = stream.unwrap();
        let job = Box::new(move || {
            handle_connection(&stream, config);
        });
        thread_pool.execute(job);
    }
}
