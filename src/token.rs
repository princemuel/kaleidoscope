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
    Comma,
    /// Argument separator in function calls
    Comment,

    // Keywords
    Def,
    Extern,

    // Literals / identifiers
    Ident(&'a str),
    Number(Number),

    // Operators
    Op(char),
    Invalid(&'a str),
    // Named constants: π, etc.
    // Constant(&'a str, f64),
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
            TokenKind::Number(v) => match v {
                Number::Int(v) => write!(f, "{v}"),
                Number::Float(v) => write!(f, "{v}"),
            },
            // TokenKind::Constant(name, _) => write!(f, "{name}"),
            TokenKind::Op(v) => write!(f, "{v}"),
            // TokenKind::Function(name) => write!(f, "{name}"),
            TokenKind::Comma => write!(f, ","),
            // TokenKind::LeftParen => write!(f, "("),
            // TokenKind::RightParen => write!(f, ")"),
            Self::Comment => write!(f, "#"),
            Self::Def => write!(f, "def"),
            Self::Extern => write!(f, "extern"),
            Self::Ident(v) | Self::Invalid(v) => write!(f, "{v}"),
        }
    }
}
