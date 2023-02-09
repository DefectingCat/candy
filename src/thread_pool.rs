use std::thread;

pub struct ThreadPool {
    pub thread_num: usize,
    pub works: Vec<Worker>,
}

impl ThreadPool {
    pub fn new(thread_num: usize) -> Self {
        let mut works = Vec::with_capacity(thread_num);
        for id in 0..thread_num {
            works.push(Worker::new(id))
        }
        Self {
            thread_num,
            works: vec![],
        }
    }
}

pub struct Worker {
    pub id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            thread: thread::spawn(|| {}),
        }
    }
}
