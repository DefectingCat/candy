use std::time::Duration;

use axum_server::{Address, Handle};
use tokio::signal;
use tracing::{debug, info};

pub async fn graceful_shutdown<A: Address>(handle: Handle<A>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
        },
        _ = terminate => {
        },
    }

    info!("Received termination signal shutting down");
    info!("Server shuting down");

    // Signal the server to shutdown using Handle.
    handle.graceful_shutdown(Some(Duration::from_secs(30)));
}

/// Parse port from host
/// if host is localhost:8080
/// return 8080
/// if host is localhost
/// return 80
pub fn parse_port_from_host(host: &str, scheme: &str) -> Option<u16> {
    // localhost:8080
    // ["localhost", "8080"]
    // localhost
    // ["localhost"]
    let host_parts = host.split(':').collect::<Vec<&str>>();
    let port = if host_parts.len() == 1 {
        match scheme {
            "http" => 80,
            "https" => 443,
            _ => {
                debug!("scheme not support");
                return None;
            }
        }
    } else {
        host_parts.get(1)?.parse::<u16>().ok()?
    };
    Some(port)
}
