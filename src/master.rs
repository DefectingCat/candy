use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

use crate::config::Config;

/// Worker 进程信息
///
/// 使用 fork() 创建子进程，父进程只持有 PID，
/// 通过 signal::kill() 进行进程管理。
pub struct Worker {
    pub pid: u32,
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

        // 等待所有 worker 优雅退出（最多等待 10 秒）
        let max_wait_ms = 10_000;
        let check_interval_ms = 100;
        let mut waited_ms = 0;

        while waited_ms < max_wait_ms {
            let all_dead = self.workers.iter().all(|worker| {
                signal::kill(Pid::from_raw(worker.pid as i32), None).is_err()
            });

            if all_dead {
                println!("All workers gracefully shut down after {}ms", waited_ms);
                return Ok(());
            }

            std::thread::sleep(Duration::from_millis(check_interval_ms));
            waited_ms += check_interval_ms;
        }

        // 强制杀死未退出的 worker
        println!("Force killing remaining workers after timeout");
        for worker in &self.workers {
            let _ = signal::kill(Pid::from_raw(worker.pid as i32), Signal::SIGKILL);
        }

        println!("All workers shut down");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_new() {
        let temp_dir = tempfile::tempdir().unwrap();
        let toml = format!(
            r#"
[server]
listen = "127.0.0.1:8080"
root = "{}"
workers = 1
"#,
            temp_dir.path().display()
        );
        let config = Config::parse(&toml).unwrap();
        let master = Master::new(config);
        assert_eq!(master.workers.len(), 0);
        assert!(!master.shutdown.load(Ordering::SeqCst));
    }

    #[test]
    fn test_shutdown_flag() {
        let temp_dir = tempfile::tempdir().unwrap();
        let toml = format!(
            r#"
[server]
listen = "127.0.0.1:8080"
root = "{}"
"#,
            temp_dir.path().display()
        );
        let config = Config::parse(&toml).unwrap();
        let master = Master::new(config);

        // 初始状态未关闭
        assert!(!master.shutdown.load(Ordering::SeqCst));

        // 设置关闭标志
        master.shutdown.store(true, Ordering::SeqCst);
        assert!(master.shutdown.load(Ordering::SeqCst));
    }
}
