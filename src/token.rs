use core::fmt;

/// A fully classified token together with its source span.
#[derive(Clone, Debug, PartialEq)]
pub struct Token<'a> {
    pub kind: TokenKind<'a>,
    pub span: Span,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenKind<'a> {
    Eof,
    Comma,
    /// Argument separator in function calls
    Comment,

    // Keywords
    Def,
    Extern,
    Else,
    For,
    If,
    In,
    Then,
    Var,

    // Literals / identifiers
    Ident(&'a str),
    Number(Number),
    // Named constants: π, etc.
    // Constant(&'a str, f64),

    // Operators
    Binary,
    LParen,
    RParen,
    Op(char),
    Unary,
    // Invalid
    Invalid(&'a str),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Number {
    Int(i64),
    Float(f64),
}

impl Number {
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        if !s.contains('.') {
            return s.parse().ok().map(Self::Int);
        }

        let v: f64 = s.parse().ok()?;
        v.is_finite().then_some(Self::Float(v))
    }
}

impl fmt::Display for TokenKind<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Comma => write!(f, ","),
            Self::Comment => write!(f, "#"),
            Self::Def => write!(f, "def"),
            Self::Extern => write!(f, "extern"),
            Self::Else => write!(f, "else"),
            Self::For => write!(f, "for"),
            Self::If => write!(f, "if"),
            Self::In => write!(f, "in"),
            Self::Then => write!(f, "then"),
            Self::Var => write!(f, "var"),

            Self::Ident(v) | Self::Invalid(v) => write!(f, "{v}"),
            Self::Number(v) => match v {
                Number::Int(v) => write!(f, "{v}"),
                Number::Float(v) => write!(f, "{v}"),
            },
            // Self::Constant(name, _) => write!(f, "{name}"),
            // Self::Function(name) => write!(f, "{name}"),
            Self::LParen => write!(f, "("),
            Self::Op(v) => write!(f, "{v}"),
            Self::RParen => write!(f, ")"),
            // Self::Binary => todo!(),
            // Self::Unary => todo!(),
            _ => unimplemented!(),
        }
    }
}
