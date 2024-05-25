#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid command")]
    InvalidCommand,

    #[error("Failed to parse input: {0}")]
    ParseError(#[from] super::resp::Error),

    #[error(transparent)]
    ServerError(#[from] super::server::tcp::Error),
}
