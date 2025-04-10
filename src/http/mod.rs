use std::time::Duration;

use axum::{Router, middleware, routing::get};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;
use tracing::{debug, info, warn};

use crate::{
    config::SettingHost,
    middlewares::{add_version, logging_route},
    utils::{shutdown, shutdown_signal},
};

pub mod error;
pub mod serve;

pub async fn make_server(host: SettingHost) -> anyhow::Result<()> {
    let mut router = Router::new();
    // find routes in config
    // convert to axum routes
    for host_route in &host.route {
        // reverse proxy
        if host_route.proxy_pass.is_some() {
            continue;
            // router = router.route(host_route.location.as_ref(), get(hello));
        }

        // static file
        if host_route.root.is_none() {
            warn!("root field not found for route: {:?}", host_route.location);
            continue;
        }
        // resister with location
        // location = "/doc"
        // route: GET /doc/*
        // resister with file path
        // index = ["index.html", "index.txt"]
        // route: GET /doc/index.html
        // route: GET /doc/index.txt
        let route_path = format!("{}/{{*path}}", host_route.location);
        debug!("registing route: {:?}", route_path);
        router = router.route(route_path.as_ref(), get(serve::serve));
        // for index in &host_route.index {
        //     let file_path = format!("{}/{}", host_route.location, index);
        //     debug!("registing route: {:?}", file_path);
        //     router = router.route(file_path.as_str(), get(serve::serve));
        // }
    }

    router = router.layer(
        ServiceBuilder::new()
            .layer(middleware::from_fn(add_version))
            .layer(TimeoutLayer::new(Duration::from_secs(host.timeout.into()))),
    );
    router = logging_route(router);

    let addr = format!("{}:{}", host.ip, host.port);
    let listener = TcpListener::bind(&addr).await?;
    info!("listening on {}", addr);

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal(shutdown))
        .await?;

    Ok(())
}
