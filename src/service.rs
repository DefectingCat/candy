use axum::{routing::get, Router};
use tokio::{net::TcpListener, signal};
use tower_http::services::ServeDir;
use tracing::{info, warn};

use crate::config::SettingHost;

/// hello world
pub async fn hello() -> String {
    format!("hello {}", env!("CARGO_PKG_NAME"))
}

impl SettingHost {
    pub async fn mk_server(self) -> anyhow::Result<()> {
        let mut router = Router::new();
        for host_route in self.route {
            let Some(host_route) = host_route.as_ref() else {
                continue;
            };
            if host_route.proxy_pass.is_some() {
                todo!();
                router = router.route(host_route.location.as_ref(), get(hello));
            }
            let Some(root) = &host_route.root else {
                warn!("root field not found");
                continue;
            };
            router = router.route_service(host_route.location.as_ref(), ServeDir::new(root));
        }

        let addr = format!("{}:{}", self.ip, self.port);
        let listener = TcpListener::bind(&addr).await?;
        info!("listening on {}", addr);

        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal(shutdown))
            .await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AppState {}

/// Asynchronously waits for a shutdown signal and executes a callback function when a signal is received.
///
/// This function listens for shutdown signals in the form of `Ctrl+C` and termination signals. When one of
/// these signals is received, it invokes the provided callback function `shutdown_cb`.
///
/// The behavior of the signal handling depends on the operating system:
///
/// - On Unix-based systems (e.g., Linux, macOS), it listens for termination signals (such as SIGTERM).
/// - On non-Unix systems (e.g., Windows), it only listens for `Ctrl+C` and ignores termination signals.
///
/// The `shutdown_cb` callback function is executed when either signal is received. This function should
/// contain the logic needed to gracefully shut down the application or perform any necessary cleanup tasks.
/// # Parameters
///
/// - `shutdown_cb`: A closure or function to call when a shutdown signal is received. The function should
///   have the signature `Fn()`. This callback is executed without any parameters.
///
/// # Errors
///
/// - If setting up the signal handlers fails, the function will panic with an error message.
///
/// # Panics
///
/// - Panics if the setup for `Ctrl+C` or termination signal handlers fails.
///
/// # Platform-specific behavior
///
/// - On Unix-based systems, termination signals are handled using the `signal` crate for Unix signals.
/// - On non-Unix systems, only `Ctrl+C` signals are handled, and termination signals are not supported.
///
/// # Future
///
/// This function returns a future that resolves when either `Ctrl+C` or a termination signal is received
/// and the callback function has been executed.
pub async fn shutdown_signal<F>(shutdown_cb: F)
where
    F: Fn(),
{
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
            shutdown_cb()
            // let _ = stop_core().map_err(log_err);
        },
        _ = terminate => {
            shutdown_cb()
        },
    }
}

fn shutdown() {
    info!("Server shuting down")
}
