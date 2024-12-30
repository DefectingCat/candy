use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
/// A tiny HTTP server.
pub struct Cli {
    /// Set a custom config file location.
    #[arg(short, long, value_name = "FILE", default_value = "./config.toml")]
    pub config: String,
}
