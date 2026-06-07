//! The Kaleidoscope Abstract Syntax Tree (aka Parse Tree)
use crate::token::Number;

/// Every expression form the language can produce.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    /// A binary operation: `lhs op rhs`.
    Binary { op: char, lhs: Box<Expr>, rhs: Box<Expr> },
    /// A function call: `name(...args)`.
    Call { name: String, args: Vec<Expr> },
    /// A numeric literal: `1`, `3.14`, etc.
    Number(Number),
    /// A variable reference: `x`, `foo`, etc.
    Variable(String),
}

/// The prototype (signature) of a function: its name and parameter names.
///
/// An `extern` declaration has a `Prototype` but no body.
#[derive(Debug, Clone, PartialEq)]
pub struct Prototype {
    pub name: String,
    pub args: Vec<String>,
}

/// A function. can be either a definition (with body) or an extern
/// (without).
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub proto: Prototype,
    pub body: Option<Expr>,
}
