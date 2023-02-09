use std::thread;

pub struct ThreadPool {
    thread_num: u32,
    threads: Vec<thread::JoinHandle<()>>,
}

impl ThreadPool {
    fn new(thread_num: u32) -> Self {
        Self {
            thread_num,
            threads,
        }
    }
}
