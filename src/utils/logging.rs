use tracing_subscriber::{
    fmt::{self},
    prelude::*,
    registry, EnvFilter,
};

pub fn init_logger() {
    let formatting_layer = fmt::layer()
        // .pretty()
        // .with_thread_ids(true)
        .with_target(false)
        .with_writer(std::io::stdout);

    let env_layer = EnvFilter::try_from_env("CANDY_LOG").unwrap_or_else(|_| "info".into());
    registry().with(env_layer).with(formatting_layer).init();
}
