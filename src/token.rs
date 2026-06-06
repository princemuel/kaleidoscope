#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenKind<'a> {
    Eof,
    // commands
    Def,
    Extern,
    // primary
    Ident(&'a str),
    Number(f64),
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
pub(crate) struct Token<'a> {
    pub kind: TokenKind<'a>,
    pub span: Span,
}
