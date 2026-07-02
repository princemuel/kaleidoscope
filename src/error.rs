use inkwell::builder::BuilderError;
use inkwell::support::LLVMString;
use thiserror::Error as ThisError;

/// The top-level error type
#[derive(Debug, ThisError)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Codegen(#[from] CodegenError),

    #[error(transparent)]
    LLVMString(#[from] LLVMString),

    #[error(transparent)]
    Parser(#[from] ParseError),

    #[error("{0}")]
    Unknown(String),
}

#[derive(Debug, ThisError)]
pub enum CodegenError {
    #[error("unknown variable: '{0}'")]
    UnknownVariable(String),

    #[error("unknown function referenced: '{0}'")]
    UnknownFunction(String),

    #[error("invalid binary operator: '{0}'")]
    InvalidBinaryOp(char),

    #[error("argument count mismatch: expected {expected}, got {got}")]
    ArgCountMismatch { expected: usize, got: usize },

    #[error("function '{0}' cannot be redefined")]
    FunctionRedefinition(String),

    #[error("function verification failed: '{0}'")]
    VerificationFailed(String),

    #[error(transparent)]
    Builder(#[from] BuilderError),

    #[error(transparent)]
    LLVMString(#[from] LLVMString),

    #[error("{0}")]
    Unknown(String),
}

#[derive(Debug, ThisError, PartialEq)]
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
