use std::io;

#[allow(clippy::enum_variant_names)]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    // from
    #[error("failed io {0}")]
    Io(#[from] io::Error),
    #[error("failed to decode toml {0}")]
    TomlDecode(#[from] toml::de::Error),
    #[error("failed to handle http {0}")]
    Http(#[from] hyper::http::Error),

    // http
    #[error("route not found {0}")]
    NotFound(String),
    #[error("internal server error {0}")]
    InternalServerError(#[from] anyhow::Error),
}

pub type Result<T, E = Error> = anyhow::Result<T, E>;
