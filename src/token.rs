#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenKind<'a> {
    // Eof, // do i really need this???
    Comma,
    Comment,
    // commands
    Def,
    Extern,
    // primary
    Ident(&'a str),
    Number(f64),
    // unknown
    Op(char),
}

/// A byte-range into the original source string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    #[must_use]
    pub fn slice<'a>(&self, src: &'a str) -> &'a str { &src[self.start..self.end] }
}

/// A fully classified token: its kind + where it lives in the source.
#[derive(Clone, Debug, PartialEq)]
pub struct Token<'a> {
    pub kind: TokenKind<'a>,
    pub span: Span,
}
