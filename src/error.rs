use std::io;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed io")]
    IoError(#[from] io::Error),
    #[error("failed to decode toml")]
    TomlDecode(#[from] toml::de::Error),
}

pub type Result<T, E = Error> = anyhow::Result<T, E>;
