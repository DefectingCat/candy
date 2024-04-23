use std::io;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed io {0}")]
    IoError(#[from] io::Error),
    #[error("failed to decode toml {0}")]
    TomlDecode(#[from] toml::de::Error),
    #[error("failed to handle http {0}")]
    HttpError(#[from] hyper::http::Error),
}

pub type Result<T, E = Error> = anyhow::Result<T, E>;
