use thiserror::Error;

#[derive(Debug, Error)]
pub enum PravdaError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serde error: {0}")]
    Serde(String),

    #[error("SQLite error: {0}")]
    Sqlite(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Invalid price: {0}")]
    InvalidPrice(String),

    #[error("Invalid fee rate")]
    InvalidFee,

    #[error("Invalid limit")]
    InvalidLimit,

    #[error("Non-finite spread")]
    NonFiniteSpread,

    #[error("Source unavailable: {0}")]
    SourceUnavailable(String),

    #[error("Configuration error: {0}")]
    Config(String),
}
