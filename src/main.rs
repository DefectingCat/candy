mod config;
mod http;
mod master;
mod router;
mod socket;
mod worker;

use config::Config;
use master::Master;

fn main() {
    // 加载配置
    let config = match Config::from_file("candy.toml") {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load config: {e}");
            std::process::exit(1);
        }
    };

    println!("Candy server starting...");
    println!("  Listen: {}", config.server.listen);
    println!("  Workers: {}", config.server.workers);
    println!("  Root: {}", config.server.root.display());

    // 创建并运行 Master
    let mut master = Master::new(config);

    if let Err(e) = master.spawn_workers() {
        eprintln!("Failed to spawn workers: {e}");
        std::process::exit(1);
    }

    if let Err(e) = master.run() {
        eprintln!("Master error: {e}");
        std::process::exit(1);
    }
}
