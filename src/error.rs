use std::io;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // from
    #[error("failed io {0}")]
    Io(#[from] io::Error),
    #[error("failed to decode toml {0}")]
    TomlDecode(#[from] toml::de::Error),
    #[error("failed to handle http {0}")]
    Http(#[from] hyper::http::Error),

    // self
    #[error("route not found {0}")]
    NotFound(String),
}

pub type Result<T, E = Error> = anyhow::Result<T, E>;
