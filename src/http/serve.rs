use axum::{
    extract::{Request, State},
    response::IntoResponse,
};
use tower::ServiceExt;
use tower_http::services::ServeDir;
use tracing::debug;

use super::{
    AppState,
    error::{RouteError, RouteResult},
};

#[axum::debug_handler]
pub async fn serve(request: Request) -> RouteResult<impl IntoResponse> {
    debug!("request: {:?}", request);
    // let host_route = app
    //     .host_route
    //     .get(&request.uri().path().to_string())
    //     .unwrap();
    // let has_html = host_route.index.iter().any(|s| s == ".html");
    // let Some(root) = host_route.root.as_ref() else {
    //     return Err(RouteError::Any(anyhow::anyhow!("root field not found")));
    // };
    // if has_html {
    //     let service = ServeDir::new(root);
    //     let res = service.oneshot(request).await?;
    //     return Ok(res);
    // } else {
    //     return Err(RouteError::Any(anyhow::anyhow!("root field not found")));
    // }
    Ok(())
}
