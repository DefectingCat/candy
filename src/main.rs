use anyhow::{Context, Result};
use tracing::{debug, info};

use crate::{config::init_config, utils::init_logger};

mod config;
mod error;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();
    let settings = init_config().with_context(|| "init config failed")?;
    debug!("settings {:?}", settings);
    info!("Hello");
    Ok(())
}
