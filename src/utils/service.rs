use std::time::Duration;

use axum_server::Handle;
use tokio::{signal, time::sleep};
use tracing::{debug, info};

// Asynchronously waits for a shutdown signal and executes a callback function when a signal is received.
//
// This function listens for shutdown signals in the form of `Ctrl+C` and termination signals. When one of
// these signals is received, it invokes the provided callback function `shutdown_cb`.
//
// The behavior of the signal handling depends on the operating system:
//
// - On Unix-based systems (e.g., Linux, macOS), it listens for termination signals (such as SIGTERM).
// - On non-Unix systems (e.g., Windows), it only listens for `Ctrl+C` and ignores termination signals.
//
// The `shutdown_cb` callback function is executed when either signal is received. This function should
// contain the logic needed to gracefully shut down the application or perform any necessary cleanup tasks.
// # Parameters
//
// - `shutdown_cb`: A closure or function to call when a shutdown signal is received. The function should
//   have the signature `Fn()`. This callback is executed without any parameters.
//
// # Errors
//
// - If setting up the signal handlers fails, the function will panic with an error message.
//
// # Panics
//
// - Panics if the setup for `Ctrl+C` or termination signal handlers fails.
//
// # Platform-specific behavior
//
// - On Unix-based systems, termination signals are handled using the `signal` crate for Unix signals.
// - On non-Unix systems, only `Ctrl+C` signals are handled, and termination signals are not supported.
//
// # Future
//
// This function returns a future that resolves when either `Ctrl+C` or a termination signal is received
// and the callback function has been executed.
// pub async fn shutdown_signal<F>(shutdown_cb: F)
// where
//     F: Fn(),
// {
//     let ctrl_c = async {
//         signal::ctrl_c()
//             .await
//             .expect("failed to install Ctrl+C handler");
//     };
//
//     #[cfg(unix)]
//     let terminate = async {
//         signal::unix::signal(signal::unix::SignalKind::terminate())
//             .expect("failed to install signal handler")
//             .recv()
//             .await;
//     };
//
//     #[cfg(not(unix))]
//     let terminate = std::future::pending::<()>();
//
//     tokio::select! {
//         _ = ctrl_c => {
//         },
//         _ = terminate => {
//         },
//     }
//
//     tracing::info!("Received termination signal shutting down");
//     shutdown_cb()
// }

pub async fn graceful_shutdown(handle: Handle) {
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

    // Print alive connection count every second.
    loop {
        sleep(Duration::from_secs(1)).await;
        debug!("alive connections: {}", handle.connection_count());
    }
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
