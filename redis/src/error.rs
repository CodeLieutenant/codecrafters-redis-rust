#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid command")]
    InvalidCommand,

    #[error("command does not exist")]
    NotExists,

    #[error("Invalid arguments given to the command {0}: {1}")]
    InvalidArguments(&'static str, &'static str),

    #[error("Failed to parse input: {0}")]
    ParseError(#[from] super::resp::Error),

    #[error("Invalid UTF8 Input: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error(transparent)]
    ServerError(#[from] super::server::tcp::Error),
}
