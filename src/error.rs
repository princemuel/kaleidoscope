/// The top-level error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Parser(#[from] ParseError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("unexpected end of input")]
    UnexpectedEof,

    #[error("expected number literal, got '{0}'")]
    ExpectedNumber(String),

    #[error("expected identifier, got '{0}'")]
    ExpectedIdent(String),

    #[error("expected '(' got '{0}'")]
    ExpectedLParen(String),

    #[error("expected ')' got '{0}'")]
    ExpectedRParen(String),

    #[error("expected ',' or ')' in argument list, got '{0}'")]
    ExpectedCommaOrRParen(String),

    #[error("expected operator in custom operator declaration, got '{0}'")]
    ExpectedOperator(String),

    #[error("expected function name in prototype, got '{0}'")]
    ExpectedPrototypeName(String),

    #[error("unknown token {0} when expecting an expression")]
    ExpectedExpr(String),

    #[error("invalid binary operator: '{0}'")]
    InvalidOperator(String),

    #[error("invalid operator precedence: {0}")]
    InvalidPrecedence(f64),

    #[error("unexpected tokens after parsed expression, starting at '{0}'")]
    TrailingTokens(String),
}
