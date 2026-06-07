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

    /// Returns the next token from stdin.
    fn lex_next(&mut self) -> Token<'a> {
        // skip whitespace
        self.advance_while(|b| b.is_ascii_whitespace());

        let start = self.cursor;

        let Some(ch) = self.peek() else { return self.simple(TokenKind::Eof, start) };

        self.advance(); // consume token

        match ch {
            b'#' => {
                self.advance_while(|b| b != b'\n' && b != b'\r');
                self.simple(TokenKind::Comment, start)
            }

            b',' => self.simple(TokenKind::Comma, start),

            b'(' => self.simple(TokenKind::LParen, start),

            b')' => self.simple(TokenKind::RParen, start),

            b if b.is_ascii_alphabetic() || b == b'_' => {
                self.advance_while(|b| b.is_ascii_alphanumeric() || b == b'_');

                let lexeme = self.slice(start);
                let kind = match lexeme {
                    "def" => TokenKind::Def,
                    "extern" => TokenKind::Extern,
                    "if" => TokenKind::If,
                    "then" => TokenKind::Then,
                    "else" => TokenKind::Else,
                    "for" => TokenKind::For,
                    "in" => TokenKind::In,
                    "unary" => TokenKind::Unary,
                    "binary" => TokenKind::Binary,
                    "var" => TokenKind::Var,
                    _ => TokenKind::Ident(lexeme),
                };

                self.simple(kind, start)
            }

            b if b.is_ascii_digit() || b == b'.' => {
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
            // b if b.is_ascii_digit() || b == b'.' => {
            //     self.advance_while(|b| b.is_ascii_digit() || b == b'.');

            //     let lexeme = self.slice(start);

            //     let kind = match Number::parse(lexeme) {
            //         Some(n) => TokenKind::Number(n),
            //         None => TokenKind::Invalid(lexeme),
            //     };

            //     self.simple(kind, start)
            // }
            b => self.simple(TokenKind::Op(char::from(b)), start),
        }
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
        while self.peek().is_some_and(&mut predicate) {
            self.advance();
        }
    }

    #[expect(dead_code)]
    fn is_eof(&self) -> bool { self.cursor >= self.source.len() }

    fn span(&self, start: usize) -> Span { Span { start, end: self.cursor } }

    fn slice(&self, start: usize) -> &'a str { &self.source[start..self.cursor] }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = TokenKind<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.lex_next();
        if token.kind == TokenKind::Eof { None } else { Some(token.kind) }
    }
}

impl FusedIterator for Lexer<'_> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex() {
        //         let source = "
        //         # Compute the x'th fibonacci number.
        // def fib(x)
        //   if x < 3 then
        //     1
        //   else
        //     fib(x - 1) + fib(x - 2)
        // ";
        //         let lexer = Lexer::new(source);

        //         for Token { kind, .. } in lexer {
        //             println!("{kind:?}");
        //         }

        //         println!("\n\n");

        //         let source = "extern sin(arg);
        // extern cos(arg);
        // extern atan2(arg1 arg2);

        // atan2(sin(.4), cos(42))
        // ";

        let source = "def foo(x y) x+y y;";
        let lexer = Lexer::new(source);

        for token in lexer {
            print!("{token:?} ");
        }
    }
}
