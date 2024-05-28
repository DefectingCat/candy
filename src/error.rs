use std::{borrow::Cow, io, num::TryFromIntError, sync::PoisonError, time::SystemTimeError};

use anyhow::anyhow;
use http::uri::InvalidUri;
use hyper::header::{InvalidHeaderValue, ToStrError};

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

    // http
    #[error("route not found {0}")]
    NotFound(Cow<'static, str>),
    #[error("internal server error {0}")]
    InternalServerError(#[from] anyhow::Error),
    #[error("invalide header value {0}")]
    InvalidHeader(#[from] InvalidHeaderValue),
    #[error("")]
    Empty,
}

pub type Result<T, E = Error> = anyhow::Result<T, E>;

impl<T> From<PoisonError<T>> for Error {
    fn from(err: PoisonError<T>) -> Self {
        Self::InternalServerError(anyhow!("global cache poisoned {err}"))
    }
}
