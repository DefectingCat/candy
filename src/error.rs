use std::{io, num::TryFromIntError, time::SystemTimeError};

use http::uri::InvalidUri;
use hyper::header::ToStrError;

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
    #[error("failed to handle system time {0}")]
    Time(#[from] SystemTimeError),
    #[error("failed to convert int {0}")]
    TryFromInt(#[from] TryFromIntError),
    #[error("failed to convert str {0}")]
    ToStr(#[from] ToStrError),
    #[error("failed to convert url {0}")]
    InvalidUri(#[from] InvalidUri),
    #[error("hyper {0}")]
    HyperError(#[from] hyper::Error),
    #[error("internal server error {0}")]
    Any(#[from] anyhow::Error),
}

pub type Result<T, E = Error> = anyhow::Result<T, E>;
