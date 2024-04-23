use std::pin::Pin;

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
    pub fn mk_server(self) -> impl Future<Output = anyhow::Result<()>> + 'static {
        let addr = format!("{}:{}", self.ip, self.port);
        #[allow(unreachable_code)]
        async move {
            let listener = TcpListener::bind(addr).await?;
            loop {
                let (stream, _) = listener.accept().await?;
                let io = TokioIo::new(stream);

                let host = self.clone();
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
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        let res = match req.uri().path() {
            "/" => mk_response(format!("home! counter ")),
            "/posts" => mk_response(format!("posts, of course! counter ")),
            "/authors" => mk_response(format!("authors extraordinare! counter")),
            // Return the 404 Not Found for other routes, and don't increment counter.
            _ => return Box::pin(async { mk_response("oh no! not found".into()) }),
        };

        Box::pin(async { res })
    }
}

async fn mk_response(s: String) -> Result<Response<Full<Bytes>>, hyper::Error> {
    Ok(Response::builder().body(Full::new(Bytes::from(s))).unwrap())
}
