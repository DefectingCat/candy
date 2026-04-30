mod config;
mod socket;

use config::Config;

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
}
