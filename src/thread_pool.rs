use anyhow::Result;
use log::{error, info};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

pub struct ThreadPool {
    pub thread_num: usize,
    pub works: Vec<Worker>,
    sender: Sender<Job>,
}

type Job = Box<dyn Send + FnOnce() + 'static>;

impl ThreadPool {
    /// Create threads.
    /// If thread number < 1 will be create
    /// threads with CPU thread.
    pub fn new(thread_num: usize) -> Self {
        let thread_num = {
            if thread_num < 1 {
                let num = num_cpus::get();
                info!("create {num} worker(s)");
                num
            } else {
                info!("create {thread_num} worker(s)");
                thread_num
            }
        };
        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut works = Vec::with_capacity(thread_num);
        for id in 0..thread_num {
            works.push(Worker::new(id, receiver.clone()))
        }
        Self {
            thread_num,
            works: vec![],
            sender,
        }
    }

    pub fn exeute(&self, job: Job) {
        match self.sender.send(job) {
            Ok(()) => info!("string send job to worker"),
            Err(err) => error!("failed to send job to worker {}", err.to_string()),
        }
    }
}

pub struct Worker {
    pub id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    pub fn new(id: usize, receiver: Arc<Mutex<Receiver<Job>>>) -> Result<Self> {
        let builder = thread::Builder::new();
        let worker_job = move || loop {
            let job = match receiver.lock() {
                Ok(lock) => match lock.recv() {
                    Ok(job) => job,
                    Err(err) => {
                        error!("worker {id} failed to get thread job {}", err.to_string());
                        Box::new(|| {})
                    }
                },
                Err(err) => {
                    error!("worker {id} failed to get thread job {}", err.to_string());
                    Box::new(|| {})
                }
            };
            info!("worker {id} received job; executing");
            job();
        };
        let thread = builder.spawn(worker_job)?;
        info!("create worker with id {id}");
        Ok(Self { id, thread })
    }
}
