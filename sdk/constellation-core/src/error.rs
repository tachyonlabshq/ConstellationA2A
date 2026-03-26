//! Error types for the Constellation SDK.
//!
//! All fallible operations in the SDK return [`Result<T>`] which uses
//! [`ConstellationError`] as the error type.

use thiserror::Error;

/// Top-level error type for the Constellation SDK.
///
/// Wraps errors from the Matrix SDK, serialization, URL parsing, and
/// domain-specific errors for configuration, connections, rooms, messages,
/// and tasks.
#[derive(Debug, Error)]
pub enum ConstellationError {
    /// An error from the Matrix SDK client.
    #[error("Matrix SDK error: {0}")]
    Matrix(#[from] matrix_sdk::Error),

    /// An HTTP-level error from the Matrix SDK.
    #[error("Matrix HTTP error: {0}")]
    MatrixHttp(#[from] matrix_sdk::HttpError),

    /// A Matrix identifier parsing error (invalid user ID, room alias, etc.).
    #[error("Matrix ID parse error: {0}")]
    MatrixId(#[from] matrix_sdk::ruma::IdParseError),

    /// An error in agent configuration (missing or invalid fields).
    #[error("Configuration error: {0}")]
    Config(String),

    /// A connection-level error (login failure, client build error, etc.).
    #[error("Connection error: {0}")]
    Connection(String),

    /// A room operation error (join failure, room not found, etc.).
    #[error("Room error: {0}")]
    Room(String),

    /// A message sending or formatting error.
    #[error("Message error: {0}")]
    Message(String),

    /// A task lifecycle error (task not found, invalid state transition, etc.).
    #[error("Task error: {0}")]
    Task(String),

    /// A JSON serialization or deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// A URL parsing error.
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
}

/// A [`Result`](std::result::Result) type alias using [`ConstellationError`].
pub type Result<T> = std::result::Result<T, ConstellationError>;
