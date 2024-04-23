#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to parse config")]
    ConfigError(#[from] config::ConfigError),
}

pub type Result<T, E = Error> = anyhow::Result<T, E>;
