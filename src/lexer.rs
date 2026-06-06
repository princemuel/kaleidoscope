use crate::token::{Span, Token, TokenKind};

/// Defines a lexer which transforms an input [`String`] into
/// a [`Token`] stream.
#[derive(Clone, Debug)]
pub struct Lexer<'a> {
    input: &'a str,
    /// The current position of the token
    cursor: usize,
}

impl<'a> Lexer<'a> {
    /// Creates a new [`Lexer`]
    #[must_use]
    pub const fn new(input: &'a str) -> Self { Self { input, cursor: 0 } }

    /// Return the next token from standard input.
    fn next_token(&mut self) -> Option<Token<'a>> {
        // skip whitespace
        self.advance_while(|b| b.is_ascii_whitespace());

        let start = self.cursor;

        let token = match self.peek()? {
            // None => Token { kind: TokenKind::Eof, span: self.span_from(start) },
            // Comment until end of line.
            b'#' => {
                self.advance_while(|b| b != b'\n' && b != b'\r');
                Token { kind: TokenKind::Comment, span: self.span_from(start) }
            }

            b',' => {
                self.advance();
                Token { kind: TokenKind::Comma, span: self.span_from(start) }
            }

            // identifier: [a-zA-Z][a-zA-Z0-9]*
            b if b.is_ascii_alphabetic() || b == b'_' => {
                self.advance_while(|b| b.is_ascii_alphanumeric() || b == b'_');

                let lexeme = self.lexeme(start);

                let kind = match lexeme {
                    "def" => TokenKind::Def,
                    "extern" => TokenKind::Extern,
                    _ => TokenKind::Ident(lexeme),
                };
                Token { kind, span: self.span_from(start) }
            }

            // Number: [0-9.]+
            b if b.is_ascii_digit() || b == b'.' => {
                // TODO: this isn’t doing sufficient error checking:
                // it will incorrectly read “1.23.45.67” and handle it as if you typed in
                // “1.23”.
                self.advance_while(|b| b.is_ascii_digit() || b == b'.');

                let n = self.lexeme(start).parse().unwrap_or(0.0);
                Token { kind: TokenKind::Number(n), span: self.span_from(start) }
            }

            b => {
                self.advance();
                Token { kind: TokenKind::Op(char::from(b)), span: self.span_from(start) }
            }
        };

        Some(token)
    }

    #[inline]
    fn peek(&self) -> Option<u8> { self.input.as_bytes().get(self.cursor).copied() }

    fn advance(&mut self) { self.cursor += 1; }

    fn advance_while(&mut self, mut predicate: impl FnMut(u8) -> bool) {
        while let Some(b) = self.peek() {
            if !predicate(b) {
                break;
            }

            self.advance();
        }
    }

    fn span_from(&self, start: usize) -> Span { Span { start, end: self.cursor } }

    fn lexeme(&self, start: usize) -> &'a str { &self.input[start..self.cursor] }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> { self.next_token() }
}
