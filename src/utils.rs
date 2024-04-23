use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter};

pub fn init_logger() {
    let formatting_layer = fmt::layer()
        // .pretty()
        .with_thread_ids(false)
        .with_target(false)
        .with_writer(std::io::stdout);

    let env_layer = EnvFilter::try_from_env("CANDY_LOG").unwrap_or_else(|_| "info".into());
    registry().with(env_layer).with(formatting_layer).init();
}

pub fn find_static_path<'a>(req_path: &'a str, location: &str) -> Option<&'a str> {
    let location_len = location.len();
    if req_path.len() < location_len {
        return None;
    }
    let path = &req_path[..location_len];
    if path == location {
        Some(&req_path[location_len - 1..])
    } else {
        None
    }
}
