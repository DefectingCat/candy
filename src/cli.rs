use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = Some("A modern, lightweight web server written in Rust.\n\nFeatures:\n- Static file serving with directory listing support\n- Reverse proxying to backend servers\n- Lua scripting (optional feature)\n- SSL/TLS encryption (HTTPS)\n- HTTP/2 support\n- Auto-reload config on file change\n- Multiple virtual hosts\n- Single binary deployment"))]
/// A modern, lightweight web server written in Rust.
pub struct Cli {
    /// Set a custom config file location.
    #[arg(short, long, value_name = "FILE", default_value = "./config.toml")]
    pub config: String,
}
