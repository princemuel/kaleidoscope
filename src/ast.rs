//! The Kaleidoscope Abstract Syntax Tree (aka Parse Tree)

use crate::token::Number;

/// Defines a primitive expression.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Expr {
    /// Defines a binary operator
    Binary { op: char, lhs: Box<Expr>, rhs: Box<Expr> },
    /// Defines a function call
    Call { name: String, args: Vec<Expr> },
    /// defines numeric literals like `1.0` or 1.
    Number(Number),
    /// Defines a variable, like `a`
    Variable(String),
}

/// Defines the prototype (name and parameters) of a function
#[derive(Debug, Clone, PartialEq)]
pub struct Prototype {
    pub name: String,
    pub args: Vec<String>,
    pub prec: u8,
    pub is_op: bool,
}

/// Defines an external or user-defined  function
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub proto: Prototype,
    pub body: Option<Expr>,
    pub is_anon: bool,
}
