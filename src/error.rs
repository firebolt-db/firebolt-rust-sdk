use thiserror::Error;

#[derive(Error, Debug)]
pub enum FireboltError {
    #[error("Authentication error: {0}")]
    Authentication(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Query error: {0}")]
    Query(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Configuration error: {0}")]
    Configuration(String),
    #[error("Header parsing error: {0}")]
    HeaderParsing(String),
    #[error("Unknown error: {0}")]
    Unknown(String),
}
