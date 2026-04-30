use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

use crate::config::Config;

/// Worker 进程信息
pub struct Worker {
    pub pid: u32,
    handle: Option<Child>,
}

/// Master 进程管理器
pub struct Master {
    config: Config,
    workers: Vec<Worker>,
    shutdown: Arc<AtomicBool>,
}

impl Master {
    /// 创建新的 Master 实例
    pub fn new(config: Config) -> Self {
        Master {
            config,
            workers: Vec::new(),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 启动所有 worker 进程
    pub fn spawn_workers(&mut self) -> std::io::Result<()> {
        let worker_count = self.config.server.workers;

        for i in 0..worker_count {
            let worker = self.spawn_worker(i)?;
            self.workers.push(worker);
        }

        println!("Spawned {} worker processes", worker_count);
        Ok(())
    }

    /// 生成单个 worker 进程
    fn spawn_worker(&self, id: usize) -> std::io::Result<Worker> {
        // 使用 fork 创建子进程
        match unsafe { nix::unistd::fork() } {
            Ok(nix::unistd::ForkResult::Parent { child }) => {
                println!("Worker {} started with PID {}", id, child);
                Ok(Worker {
                    pid: child.as_raw() as u32,
                    handle: None, // fork 后父进程没有 Child handle
                })
            }
            Ok(nix::unistd::ForkResult::Child) => {
                // 子进程：运行 worker 逻辑
                crate::worker::run(&self.config)?;
                std::process::exit(0);
            }
            Err(e) => Err(std::io::Error::from_raw_os_error(e as i32)),
        }
    }

    /// 运行主循环，等待信号
    pub fn run(&mut self) -> std::io::Result<()> {
        // 设置信号处理
        self.setup_signal_handlers()?;

        println!("Master process running, waiting for signals...");

        // 主循环
        while !self.shutdown.load(Ordering::SeqCst) {
            // 检查 worker 状态
            self.check_workers()?;

            // 短暂休眠
            std::thread::sleep(Duration::from_millis(100));
        }

        // 关闭所有 worker
        self.shutdown_workers()?;

        println!("Master process exiting");
        Ok(())
    }

    /// 设置信号处理器
    fn setup_signal_handlers(&self) -> std::io::Result<()> {
        let shutdown = self.shutdown.clone();

        // SIGTERM / SIGINT: 优雅关闭
        ctrlc::set_handler(move || {
            println!("Received shutdown signal");
            shutdown.store(true, Ordering::SeqCst);
        })
        .map_err(|e| std::io::Error::other(e.to_string()))?;

        Ok(())
    }

    /// 检查 worker 进程状态
    fn check_workers(&mut self) -> std::io::Result<()> {
        // 检查是否有 worker 意外退出
        let mut dead_workers = Vec::new();

        for (i, worker) in self.workers.iter().enumerate() {
            // 使用 kill(pid, 0) 检查进程是否存在
            let result = signal::kill(Pid::from_raw(worker.pid as i32), None);

            if result.is_err() {
                println!("Worker {} (PID {}) has died", i, worker.pid);
                dead_workers.push(i);
            }
        }

        // 重启死亡的 worker
        for i in dead_workers.into_iter().rev() {
            self.workers.remove(i);
            let worker = self.spawn_worker(i)?;
            self.workers.push(worker);
        }

        Ok(())
    }

    /// 关闭所有 worker 进程
    fn shutdown_workers(&mut self) -> std::io::Result<()> {
        println!("Shutting down {} workers...", self.workers.len());

        // 发送 SIGTERM 给所有 worker
        for worker in &self.workers {
            let _ = signal::kill(Pid::from_raw(worker.pid as i32), Signal::SIGTERM);
        }

        // 等待所有 worker 退出
        std::thread::sleep(Duration::from_millis(100));

        println!("All workers shut down");
        Ok(())
    }
}
