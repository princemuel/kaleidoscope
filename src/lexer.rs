use core::iter::FusedIterator;

use crate::token::{Span, Token, TokenKind};

/// Transforms a source string into a stream of [`Token`]s.
#[derive(Clone, Debug)]
pub struct Lexer<'a> {
    source: &'a str,
    /// Current byte position in the source.
    cursor: usize,
}

impl<'a> Lexer<'a> {
    #[must_use]
    pub const fn new(source: &'a str) -> Self { Self { source, cursor: 0 } }

    /// Lex and return the next [`Token`], including its [`Span`].
    fn lex_next(&mut self) -> Token<'a> {
        // Skip whitespace (including newlines).
        self.advance_while(|b| b.is_ascii_whitespace());

        let start = self.cursor;

        let Some(ch) = self.peek() else {
            return self.simple(TokenKind::Eof, start);
        };

        self.advance(); // consume the leading byte

        match ch {
            b'#' => {
                self.advance_while(|b| b != b'\n' && b != b'\r');
                self.simple(TokenKind::Comment, start)
            }

            b',' => self.simple(TokenKind::Comma, start),
            b'(' => self.simple(TokenKind::LParen, start),
            b')' => self.simple(TokenKind::RParen, start),

            // Identifiers and keywords: [a-zA-Z_][a-zA-Z0-9_]*
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

            b if b.is_ascii_digit() => {
                self.advance_while(|b| b.is_ascii_digit());

                if self.peek() == Some(b'.')
                    && self.peek_ahead(1).is_some_and(|b| b.is_ascii_digit())
                {
                    self.advance(); // consume '.'
                    self.advance_while(|b| b.is_ascii_digit());
                }

                let lexeme = self.slice(start);
                let kind = match lexeme.parse() {
                    Ok(n) => TokenKind::Number(n),
                    Err(_) => TokenKind::Invalid(lexeme),
                };
                self.simple(kind, start)
            }

            b'.' => {
                if self.peek().is_some_and(|b| b.is_ascii_digit()) {
                    self.advance_while(|b| b.is_ascii_digit());
                    let lexeme = self.slice(start);
                    let kind = match lexeme.parse() {
                        Ok(n) => TokenKind::Number(n),
                        Err(_) => TokenKind::Invalid(lexeme),
                    };
                    self.simple(kind, start)
                } else {
                    self.simple(TokenKind::Op('.'), start)
                }
            }

            b => self.simple(TokenKind::Op(char::from(b)), start),
        }
    }

    fn simple(&self, kind: TokenKind<'a>, start: usize) -> Token<'a> {
        Token { kind, span: self.span(start) }
    }
}

impl<'a> Lexer<'a> {
    #[inline]
    fn peek(&self) -> Option<u8> { self.peek_ahead(0) }

    #[inline]
    fn peek_ahead(&self, n: usize) -> Option<u8> {
        self.source.as_bytes().get(self.cursor + n).copied()
    }

    #[inline]
    fn advance(&mut self) { self.cursor += 1; }

    fn advance_while(&mut self, mut predicate: impl FnMut(u8) -> bool) {
        while self.peek().is_some_and(&mut predicate) {
            self.advance();
        }
    }

    fn span(&self, start: usize) -> Span { Span { start, end: self.cursor } }

    fn slice(&self, start: usize) -> &'a str { &self.source[start..self.cursor] }
}

/// Yields [`Token`] (kind + span). Stops at EOF (exclusive).
///
/// Use this when you need source positions for diagnostics.
impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let token = self.lex_next();
            match token.kind {
                TokenKind::Eof => return None,
                TokenKind::Comment => {}
                _ => return Some(token),
            }
        }
    }
}

impl FusedIterator for Lexer<'_> {}

/// Convenience iterator that yields only [`TokenKind`], discarding spans.
/// If you need spans for diagnostics, use the [`Token`] iterator.
pub struct TokenKinds<'a>(Lexer<'a>);

impl<'a> Lexer<'a> {
    /// Returns an iterator that yields [`TokenKind`] without spans.
    #[must_use]
    pub fn tokens(self) -> TokenKinds<'a> { TokenKinds(self) }
}

impl<'a> Iterator for TokenKinds<'a> {
    type Item = TokenKind<'a>;

    fn next(&mut self) -> Option<Self::Item> { self.0.next().map(|t| t.kind) }
}

impl FusedIterator for TokenKinds<'_> {}

#[cfg(test)]
mod tests {
    use core::assert_matches;

    use super::*;

    fn tokenize(src: &str) -> Vec<TokenKind<'_>> { Lexer::new(src).tokens().collect() }

    #[test]
    fn lex_simple_def() {
        let tokens = tokenize("def foo(x y) x+y y;");
        // def foo ( x y ) x + y y ;
        assert_matches!(tokens[0], TokenKind::Def);
        assert_matches!(tokens[1], TokenKind::Ident("foo"));
        assert_matches!(tokens[2], TokenKind::LParen);
        assert_matches!(tokens[3], TokenKind::Ident("x"));
        assert_matches!(tokens[4], TokenKind::Ident("y"));
        assert_matches!(tokens[5], TokenKind::RParen);
        assert_matches!(tokens[6], TokenKind::Ident("x"));
        assert_matches!(tokens[7], TokenKind::Op('+'));
        assert_matches!(tokens[8], TokenKind::Ident("y"));
        assert_matches!(tokens[9], TokenKind::Ident("y"));
        assert_matches!(tokens[10], TokenKind::Op(';'));
    }

    #[test]
    fn lex_dot_led_float() {
        // The C++ version parses ".4" as 0.4; we must match that.
        let tokens = tokenize(".4");
        assert_eq!(tokens, vec![TokenKind::Number(0.4)]);
    }

    #[test]
    fn lex_integer_then_dot_op() {
        // "1." should give Int(1) followed by Op('.'), not a float.
        let tokens = tokenize("1.");
        assert_eq!(tokens[0], TokenKind::Number(1.0));
        assert_eq!(tokens[1], TokenKind::Op('.'));
    }

    #[test]
    fn lex_comment_skipped() {
        // Comments should not appear in the token stream.
        let tokens = tokenize("# this is a comment\ndef");
        assert_eq!(tokens, vec![TokenKind::Def]);
    }

    #[test]
    fn lex_call_with_float_arg() {
        let tokens = tokenize("foo(y, 4.0)");
        assert_matches!(tokens[0], TokenKind::Ident("foo"));
        assert_matches!(tokens[1], TokenKind::LParen);
        assert_matches!(tokens[2], TokenKind::Ident("y"));
        assert_matches!(tokens[3], TokenKind::Comma);
        assert_matches!(tokens[4], TokenKind::Number(4.0));
        assert_matches!(tokens[5], TokenKind::RParen);
    }
}
