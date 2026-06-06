#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Lexer(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
