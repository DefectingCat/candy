use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CandyError {
    #[error("can not parse target")]
    Parse(String),

    #[error("page not found")]
    NotFound(#[from] io::Error),

    #[error("invalid header (expected {expected:?}, found {found:?})")]
    InvalidHeader { expected: String, found: String },

    #[error("unknown data store error")]
    Unknown,
}
