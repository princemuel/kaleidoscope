use core::iter::FusedIterator;

use crate::token::{Number, Span, Token, TokenKind};

/// Defines a lexer which transforms an input string into a token stream.
#[derive(Clone, Debug)]
pub struct Lexer<'a> {
    source: &'a str,
    /// Current byte position in the source.
    cursor: usize,
}

impl<'a> Lexer<'a> {
    #[must_use]
    pub const fn new(source: &'a str) -> Self { Self { source, cursor: 0 } }

    fn lex_next(&mut self) -> Option<Token<'a>> {
        // skip whitespace
        self.advance_while(|b| b.is_ascii_whitespace());

        if self.is_eof() {
            return None;
        }

        let start = self.cursor;

        let token = match self.peek()? {
            b'#' => {
                self.advance(); //#
                self.advance_while(|b| b != b'\n' && b != b'\r');
                self.simple(TokenKind::Comment, start)
            }

            b',' => {
                self.advance();
                self.simple(TokenKind::Comma, start)
            }

            b if b.is_ascii_alphabetic() || b == b'_' => {
                self.advance_while(|b| b.is_ascii_alphanumeric() || b == b'_');

                let lexeme = self.slice(start);
                let kind = match lexeme {
                    "def" => TokenKind::Def,
                    "extern" => TokenKind::Extern,
                    _ => TokenKind::Ident(lexeme),
                };

                self.simple(kind, start)
            }

            b if b.is_ascii_digit() => {
                self.advance_while(|b| b.is_ascii_digit());

                if self.peek() == Some(b'.')
                    && self.peek_ahead(1).is_some_and(|b| b.is_ascii_digit())
                {
                    self.advance(); // consume '.'

                    self.advance_while(|b| b.is_ascii_digit());
                }

                let lexeme = self.slice(start);

                let kind = match Number::parse(lexeme) {
                    Some(n) => TokenKind::Number(n),
                    None => TokenKind::Invalid(lexeme),
                };

                self.simple(kind, start)
            }

            b => {
                self.advance();
                self.simple(TokenKind::Op(char::from(b)), start)
            }
        };

        Some(token)
    }

    fn simple(&self, kind: TokenKind<'a>, start: usize) -> Token<'a> {
        Token { kind, span: self.span(start) }
    }
}

impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<u8> { self.peek_ahead(0) }

    fn peek_ahead(&self, n: usize) -> Option<u8> {
        self.source.as_bytes().get(self.cursor + n).copied()
    }

    fn advance(&mut self) { self.cursor += 1; }

    fn advance_while(&mut self, mut predicate: impl FnMut(u8) -> bool) {
        while let Some(b) = self.peek() {
            if !predicate(b) {
                break;
            }

            self.advance();
        }
    }

    fn is_eof(&self) -> bool { self.cursor >= self.source.len() }

    fn span(&self, start: usize) -> Span { Span { start, end: self.cursor } }

    fn slice(&self, start: usize) -> &'a str { &self.source[start..self.cursor] }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> { self.lex_next() }
}

impl FusedIterator for Lexer<'_> {}
