use anyhow::{Context, Result};

use http_body_util::Full;
use hyper::{body::Bytes, server::conn::http1, service::service_fn, Request, Response};
use hyper_util::rt::TokioIo;
use tokio::{join, net::TcpListener};
use tracing::{debug, error, info};

use crate::{config::init_config, utils::init_logger};

mod config;
mod error;
mod utils;

static INDEX1: &[u8] = b"The 1st service!";
async fn index1(_: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, hyper::Error> {
    Ok(Response::new(Full::new(Bytes::from(INDEX1))))
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();
    let settings = init_config().with_context(|| "init config failed")?;
    debug!("settings {:?}", settings);

    let addr = "0.0.0.0:4000";
    #[allow(unreachable_code)]
    let server1 = async move {
        let listener = TcpListener::bind(addr).await?;
        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);

            tokio::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(index1))
                    .await
                {
                    error!("Serving connection: {:?}", err);
                };
            });
        }
        anyhow::Ok(())
    };

    info!("Server started");

    let (res,) = join!(server1);
    res?;

    Ok(())
}
