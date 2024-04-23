use anyhow::Result;
use tracing::info;

use crate::utils::init_logger;

mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();
    println!("Hello World");
    info!("Hello");
    Ok(())
}
