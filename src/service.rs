use std::time::Duration;

use axum::{Router, middleware};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, timeout::TimeoutLayer};
use tracing::{info, warn};

use crate::{
    config::SettingHost,
    middlewares::{add_version, logging_route},
    utils::{shutdown, shutdown_signal},
};

impl SettingHost {
    pub async fn mk_server(self) -> anyhow::Result<()> {
        let mut router = Router::new();
        // find routes in config
        // convert to axum routes
        for host_route in self.route {
            let Some(host_route) = host_route.as_ref() else {
                continue;
            };
            // reverse proxy
            if host_route.proxy_pass.is_some() {
                continue;
                // router = router.route(host_route.location.as_ref(), get(hello));
            }

            // static file
            let Some(root) = &host_route.root else {
                warn!("root field not found");
                continue;
            };
            router = router.route_service(host_route.location.as_ref(), ServeDir::new(root));
        }

        router = router.layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn(add_version))
                .layer(TimeoutLayer::new(Duration::from_secs(self.timeout.into()))),
        );
        router = logging_route(router);

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
