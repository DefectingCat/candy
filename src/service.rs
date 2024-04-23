use std::pin::Pin;

use crate::error::{Error, Result};
use futures_util::Future;
use http_body_util::Full;
use hyper::{
    body::{Bytes, Incoming as IncomingBody},
    server::conn::http1,
    service::Service,
    Request, Response,
};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tracing::error;

use crate::config::SettingHost;

impl SettingHost {
    pub fn mk_server(&self) -> impl Future<Output = anyhow::Result<()>> + 'static {
        let addr = format!("{}:{}", self.ip, self.port);
        let host = self.clone();
        #[allow(unreachable_code)]
        async move {
            let listener = TcpListener::bind(addr).await?;
            loop {
                let host = host.clone();
                let (stream, _) = listener.accept().await?;
                let io = TokioIo::new(stream);

                tokio::spawn(async move {
                    if let Err(err) = http1::Builder::new().serve_connection(io, host).await {
                        error!("Serving connection: {:?}", err);
                    };
                });
            }
            anyhow::Ok(())
        }
    }
}

impl Service<Request<IncomingBody>> for SettingHost {
    type Response = Response<Full<Bytes>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        Box::pin(mk_response(req))
    }
}

pub async fn mk_response(req: Request<IncomingBody>) -> Result<Response<Full<Bytes>>> {
    // let route = &self.route;

    let req_path = req.uri().path();
    let res = match req_path {
        // Return the 404 Not Found for other routes.
        _ => "404",
    };
    let res = Response::builder().body(Full::new(Bytes::from(res)))?;

    Ok(res)
}
