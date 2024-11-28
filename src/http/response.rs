use std::{
    path::{Path, PathBuf},
    str::FromStr,
    time::{Duration, UNIX_EPOCH},
};

use crate::{
    config::{SettingHost, SettingRoute},
    consts::{NAME, VERSION},
    error::{Error, Result},
    get_settings,
    http::client,
    utils::{
        compress::{stream_compress, CompressType},
        find_route, parse_assets_path,
    },
};

use anyhow::{anyhow, Context};
use futures_util::TryStreamExt;
use http::{response::Builder, Method};
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::{
    body::{Bytes, Frame, Incoming},
    Request, Response, StatusCode,
};

use tokio::{
    fs::File,
    io::{AsyncBufRead, BufReader},
    select,
};
use tokio_util::io::ReaderStream;
use tracing::{debug, error, instrument};

/// HTTP handler
#[derive(Debug)]
pub struct CandyHandler<'req> {
    /// Request from hyper
    pub req: &'req Request<Incoming>,
    /// Hyper response
    pub res: Builder,
    /// Config host field
    host: &'static SettingHost,
    /// Router
    router: Option<&'req SettingRoute>,
    /// Current request's assets path
    assets_path: Option<&'req str>,
}

pub type CandyBody<T, E = Error> = BoxBody<T, E>;
type CandyResponse = Result<Response<CandyBody<Bytes>>>;
impl<'req> CandyHandler<'req> {
    /// Create a new handler with hyper incoming request
    pub fn new(req: &'req Request<Incoming>, host: &'static SettingHost) -> Self {
        Self {
            req,
            res: Response::builder(),
            host,
            router: None,
            assets_path: None,
        }
    }

    /// Traverse the headers from config
    /// add to response
    pub fn add_headers(&mut self) -> Result<()> {
        let headers = self
            .res
            .headers_mut()
            .ok_or(Error::InternalServerError(anyhow!("build response failed")))?;
        let server = format!("{}/{}", NAME, VERSION);
        headers.insert("Server", server.parse()?);
        // config headers overrite
        if let Some(c_headers) = &self.host.headers {
            for (k, v) in c_headers {
                headers.insert(k.as_str(), v.parse()?);
            }
        }
        Ok(())
    }

    /// Handle static file or reverse proxy
    pub async fn handle(mut self) -> CandyResponse {
        let req_path = self.req.uri().path();
        // find route path
        let (router, assets_path) = find_route(req_path, &self.host.route_map)?;
        self.router = Some(router);
        self.assets_path = Some(assets_path);

        // reverse proxy
        if router.proxy_pass.is_some() {
            self.proxy().await
        } else {
            // static file
            self.file().await
        }
    }

    /// Handle reverse proxy
    ///
    /// Only use with the `proxy_pass` field in config
    pub async fn proxy(self) -> CandyResponse {
        let (router, assets_path) = (
            self.router
                .ok_or(Error::NotFound("handler router is empty".into()))?,
            self.assets_path
                .ok_or(Error::NotFound("handler assets_path is empty".into()))?,
        );
        let (req, res) = (self.req, self.res);

        // check on outside
        let proxy = router.proxy_pass.as_ref().ok_or(Error::Empty)?;
        let path_query = req.uri().query().unwrap_or(assets_path);

        let uri: hyper::Uri = format!("{}{}", proxy, path_query)
            .parse()
            .with_context(|| format!("parse proxy uri failed: {}", proxy))?;

        let host = uri.host().ok_or(Error::InternalServerError(anyhow!(
            "proxy pass host incorrect"
        )))?;
        let uri = uri.clone();
        let timeout = router.proxy_timeout;
        let body = select! {
            body = client::get(uri) => {
                body.with_context(|| "proxy body error")?
            }
            _ = tokio::time::sleep(Duration::from_secs(timeout.into())) => {
                return Err(anyhow!("connect upstream {host:?} timeout").into());
            }
        };
        let res_body = res.body(body)?;
        Ok(res_body)
    }

    /// Handle static files,
    /// try find static file from local path
    ///
    /// Only use with the `proxy_pass` field not in config
    pub async fn file(self) -> CandyResponse {
        let (router, assets_path) = (
            self.router
                .ok_or(Error::NotFound("handler router is empty".into()))?,
            self.assets_path
                .ok_or(Error::NotFound("handler assets_path is empty".into()))?,
        );
        let (req, res) = (self.req, self.res);

        let req_method = req.method();

        // find resource local file path
        let mut path = None;
        for index in router.index.iter() {
            if let Some(root) = &router.root {
                let p = parse_assets_path(assets_path, root, index);
                if Path::new(&p).exists() {
                    path = Some(p);
                    break;
                }
            }
        }
        let path = match path {
            Some(p) => p,
            None => {
                return handle_not_found(req, res, router, "").await;
            }
        };

        // http method handle
        let res = match *req_method {
            Method::GET => handle_get(req, res, &path).await?,
            Method::POST => handle_get(req, res, &path).await?,
            // Return the 404 Not Found for other routes.
            _ => {
                if let Some(err_page) = &router.error_page {
                    let res = res.status(err_page.status);
                    handle_get(req, res, &err_page.page).await?
                } else {
                    not_found()
                }
            }
        };
        Ok(res)
    }
}

/// Open local file and check last modified time,
/// Then determine stream file or use cache file
///
/// ## Arguments
///
/// `path`: local file path
pub async fn open_file(path: &str) -> Result<File> {
    // Open file for reading
    let file = File::open(path).await;
    let file = match file {
        Ok(f) => f,
        Err(err) => {
            error!("Unable to open file {err}");
            return Err(Error::NotFound(format!("path not found {}", path).into()));
        }
    };
    Ok(file)
}

/// Open then use `ReaderStream` to stream to client.
/// Stream a file more suitable for large file, but its slower than read file to memory.
pub async fn stream_file<R>(file: R) -> CandyBody<Bytes>
where
    R: AsyncBufRead + Sync + Send + 'static,
{
    // Wrap to a tokio_util::io::ReaderStream
    let reader_stream = ReaderStream::new(file);
    // Convert to http_body_util::BoxBody
    let stream_body = StreamBody::new(reader_stream.map_ok(Frame::data));
    // let boxed_body = stream_body.map_err(|e| Error::IoError(e)).boxed();
    BodyExt::map_err(stream_body, Error::Io).boxed()
}

// pub async fn read_file_bytes(file: &mut File, size: u64) -> Result<Vec<u8>> {
//     let mut buffer = vec![0u8; size.try_into()?];
//     file.read_exact(&mut buffer[..]).await?;
//     Ok(buffer)
// }

// Open local file to memory
// pub async fn read_file(file: &mut File, size: u64) -> Result<CandyBody<Bytes>> {
//     let bytes = read_file_bytes(file, size).await?;
//     let body = Full::new(bytes.into()).map_err(|e| match e {}).boxed();
//     Ok(body)
// }

// HTTP status code 404
static NOT_FOUND: &[u8] = b"Not Found";
pub fn not_found() -> Response<CandyBody<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(NOT_FOUND.into()).map_err(|e| match e {}).boxed())
        .unwrap()
}

static INTERNAL_SERVER_ERROR: &[u8] = b"Internal Server Error";
pub fn internal_server_error() -> Response<CandyBody<Bytes>> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(
            Full::new(INTERNAL_SERVER_ERROR.into())
                .map_err(|e| match e {})
                .boxed(),
        )
        .unwrap()
}

// HTTP methods
/// handle http get method
/// read static file and check If-None-Match cache
#[instrument(level = "debug")]
pub async fn handle_get(
    req: &Request<Incoming>,
    mut res: Builder,
    path: &str,
) -> Result<Response<CandyBody<Bytes>>> {
    use CompressType::*;
    use Error::*;

    let headers = res
        .headers_mut()
        .ok_or(InternalServerError(anyhow!("build response failed")))?;

    // file bytes
    let file = open_file(path).await?;
    // file info
    let metadata = file.metadata().await?;
    let size = metadata.len();
    let last_modified = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();
    let etag = format!("{last_modified}-{size}");
    let extension = PathBuf::from_str(path).map_err(|err| InternalServerError(anyhow!(err)))?;
    let extension = extension
        .extension()
        .ok_or(InternalServerError(anyhow!("read file extension failed")))?;

    let settings = get_settings()?;
    let content_type = settings.types.get(
        extension
            .to_str()
            .ok_or(InternalServerError(anyhow!("read file extension failed")))?,
    );
    headers.insert(
        "Content-Type",
        content_type.unwrap_or(&settings.default_type).parse()?,
    );
    headers.insert("Etag", etag.parse()?);

    // check cache
    let if_none_match = req.headers().get("If-None-Match");
    match if_none_match {
        Some(inm) if *inm == *etag => {
            let res = res.status(304);
            let body = Full::new(vec![].into()).map_err(|e| match e {}).boxed();
            return Ok(res.body(body)?);
        }
        _ => {}
    }

    let file_reader = BufReader::new(file);
    // prepare compress
    let accept_encoding = req.headers().get("Accept-Encoding");
    let boxed_body = match accept_encoding {
        Some(accept) => {
            let accept = accept.to_str()?;
            debug!("Accept-Encoding {}", accept);
            match accept {
                str if str.contains("zstd") => {
                    headers.insert("Content-Encoding", "zstd".parse()?);
                    stream_compress(Zstd, file_reader)
                }
                str if str.contains("gzip") => {
                    headers.insert("Content-Encoding", "gzip".parse()?);
                    stream_compress(Gzip, file_reader)
                }
                str if str.contains("deflate") => {
                    headers.insert("Content-Encoding", "deflate".parse()?);
                    stream_compress(Deflate, file_reader)
                }
                str if str.contains("br") => {
                    headers.insert("Content-Encoding", "br".parse()?);
                    stream_compress(Brotli, file_reader)
                }
                _ => stream_file(file_reader).await,
            }
        }
        None => stream_file(file_reader).await,
    };

    Ok(res.body(boxed_body)?)
}

pub async fn handle_not_found(
    req: &Request<Incoming>,
    res: Builder,
    router: &SettingRoute,
    assets_path: &str,
) -> Result<Response<CandyBody<Bytes>>> {
    let res = if let Some(err_page) = &router.error_page {
        let res = res.status(err_page.status);
        if let Some(root) = &router.root {
            let path = parse_assets_path(assets_path, root, &err_page.page);
            handle_get(req, res, &path).await?
        } else {
            not_found()
        }
    } else {
        not_found()
    };
    Ok(res)
}

/// Follow http status 301
pub async fn follow_moved() {}
