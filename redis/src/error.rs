#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ServerError(#[from] super::server::tcp::Error),

    #[error("Client error: {0}")]
    Client(#[from] crate::server::handler::Error),

    #[error("Parser error: {0}")]
    Parse(#[from] crate::parser::Error)
}
