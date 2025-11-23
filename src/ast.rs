//! The Kaleidoscope Abstract Syntax Tree (aka Parse Tree)

/// ExprAST - Base for all expression nodes.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Expr {
    Binary {
        op:  char,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Number(f64),
    Variable(String),
}

/// PrototypeAST - This represents the "prototype" for a function,
/// which captures its name, and its argument names (thus implicitly the number
/// of arguments the function takes).
#[derive(Debug, Clone, PartialEq)]
pub struct Prototype {
    pub name:  String,
    pub args:  Vec<String>,
    pub prec:  usize,
    pub is_op: bool,
}

/// FunctionAST - This represents a function definition itself.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub proto:   Prototype,
    pub body:    Option<Expr>,
    pub is_anon: bool,
}
