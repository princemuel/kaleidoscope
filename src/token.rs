/// A fully classified token together with its source span.
#[derive(Clone, Debug, PartialEq)]
pub struct Token<'a> {
    pub kind: TokenKind<'a>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenKind<'a> {
    Comma,
    Comment,

    // Keywords
    Def,
    Extern,

    // Literals / identifiers
    Ident(&'a str),
    Number(f64),

    // Operators
    Op(char),
    Invalid(&'a str),
}

/// A byte-range into the original source string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

// #[derive(Clone, Debug, PartialEq)]
// pub struct Diagnostic<'a> {
//     pub message: &'a str,
//     pub span: Span,
//     pub severity: Severity,
// }

// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
// pub enum Severity {
//     Error,
//     Warning,
// }
