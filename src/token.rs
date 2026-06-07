use core::fmt;

/// A fully classified token together with its source span.
#[derive(Clone, Debug, PartialEq)]
pub struct Token<'a> {
    /// Every distinct kind of token the lexer can produce.
    pub kind: TokenKind<'a>,
    /// A byte-range into the original source string. Used for diagnostics.
    pub span: Span,
}

/// A byte-range into the original source string. Used for diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// Every distinct kind of token the lexer can produce.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenKind<'a> {
    Eof,
    Comma,
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

    // Extension keywords (for later chapters of the tutorial)
    Binary,
    Unary,

    // Literals / identifiers
    /// An identifier, borrowed zero-copy from the source string.
    Ident(&'a str),
    Number(f64),
    // Named constants: π, etc.
    // Constant(&'a str, f64),

    // Punctuation
    LParen,
    RParen,

    // A single-character operator (+, -, *, /, <, =, …)
    Op(char),

    // Emitted when the lexer cannot classify a byte sequence.
    Invalid(&'a str),
}

impl fmt::Display for TokenKind<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eof => write!(f, "<eof>"),
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
            Self::Binary => write!(f, "binary"),
            Self::Unary => write!(f, "unary"),
            // Ident and Invalid both display their text content.
            // The caller (error messages) determines whether to label it.
            Self::Ident(v) | Self::Invalid(v) => write!(f, "{v}"),
            Self::Number(v) => write!(f, "{v}"),
            Self::LParen => write!(f, "("),
            Self::RParen => write!(f, ")"),
            Self::Op(v) => write!(f, "{v}"),
        }
    }
}
