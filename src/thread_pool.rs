use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use anyhow::Result;
use log::{debug, error, info};

#[derive(Debug)]
pub struct ThreadPool {
    pub thread_num: usize,
    pub workers: Vec<Worker>,
    sender: Option<Sender<Job>>,
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
                info!("Create {num} worker(s)");
                num
            } else {
                info!("Create {thread_num} worker(s)");
                thread_num
            }
        };
        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(thread_num);
        for id in 0..thread_num {
            workers.push(Worker::new(id, Arc::clone(&receiver)).unwrap())
        }
        Self {
            thread_num,
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute(&self, job: Job) {
        match self.sender.as_ref() {
            Some(sender) => match sender.send(job) {
                Ok(()) => debug!("Starting send job to worker"),
                Err(err) => error!("Failed to send job to worker {}", err.to_string()),
            },
            None => error!("Can not get sender"),
        }
    }

    pub fn exit(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            info!("Shutting down worker {}", worker.id);

            if let Some(worker) = worker.thread.take() {
                worker.join().unwrap();
            }
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.exit();
    }
}

#[derive(Debug)]
pub struct Worker {
    pub id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    pub fn new(id: usize, receiver: Arc<Mutex<Receiver<Job>>>) -> Result<Self> {
        let builder = thread::Builder::new();
        let worker_job = move || loop {
            let job = match receiver.lock() {
                Ok(lock) => match lock.recv() {
                    Ok(job) => job,
                    Err(_) => {
                        // error!(
                        //     "Worker {id} failed to get thread job {}; shutting down",
                        //     err.to_string()
                        // );
                        break;
                    }
                },
                Err(err) => {
                    error!(
                        "Worker {id} failed to get thread job {}; shutting down",
                        err.to_string()
                    );
                    break;
                }
            };
            debug!("Worker {id} received job; executing");
            job();
        };
        let thread = builder.spawn(worker_job)?;
        info!("Create worker with id {id}");
        Ok(Self {
            id,
            thread: Some(thread),
        })
    }
}
