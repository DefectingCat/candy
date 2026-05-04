mod compress;
mod config;
mod http;
mod http2;
mod logging;
mod master;
mod router;
mod sendfile;
mod socket;
mod tls;
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
    println!("  HTTPS: {}", config.server.https_listen);
    if let Some(http_addr) = config.server.http_listen {
        println!("  HTTP: {} (redirect to HTTPS)", http_addr);
    }
    println!("  Workers: {}", config.server.workers);
    println!("  Root: {}", config.server.root.display());
    if let Some(tls) = &config.tls {
        println!("  TLS: enabled ({})", tls.cert.display());
    } else {
        println!("  TLS: disabled");
    }

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
