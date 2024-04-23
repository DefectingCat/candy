use std::io;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed io {0}")]
    Io(#[from] io::Error),
    #[error("failed to decode toml {0}")]
    TomlDecode(#[from] toml::de::Error),
    #[error("failed to handle http {0}")]
    Http(#[from] hyper::http::Error),
}

pub type Result<T, E = Error> = anyhow::Result<T, E>;
