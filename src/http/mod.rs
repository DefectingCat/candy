use std::time::Duration;

use axum::{Router, middleware, routing::get};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;
use tracing::{info, warn};

use crate::{
    config::{HostRouteMap, SettingHost, host_route_map},
    middlewares::{add_version, logging_route},
    utils::{shutdown, shutdown_signal},
};

pub mod error;
pub mod serve;

impl SettingHost {
    pub async fn mk_server(self) -> anyhow::Result<()> {
        let app_state = AppState {
            host_route: host_route_map(self.route),
        };
        let mut router = Router::new().with_state(app_state);
        // find routes in config
        // convert to axum routes
        for host_route in self.route {
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
            router = router.route(host_route.location.as_ref(), get(serve::serve));
            // Nesting at the root is no longer supported. Use fallback_service instead.
            // if host_route.location == "/" {
            //     router = router.fallback_service(ServeDir::new(root));
            // } else {
            //     router = router.nest_service(host_route.location.as_ref(), ServeDir::new(root));
            // }
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
pub struct AppState {
    host_route: HostRouteMap,
}
