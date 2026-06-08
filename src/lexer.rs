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
    use core::{assert_matches, f64};

    use super::*;

    /// Collect all `TokenKind`s from a source string, comments and EOF
    /// excluded.
    fn tokenize(src: &str) -> Vec<TokenKind<'_>> { Lexer::new(src).tokens().collect() }

    /// Collect full `Token`s (kind + span) from a source string.
    fn tokenize_full(src: &str) -> Vec<Token<'_>> { Lexer::new(src).collect() }

    /// Assert that a source string produces exactly one token of the given
    /// kind.
    macro_rules! single {
    ($src:expr, $pat:pat) => {{
        let tokens = tokenize($src);
        assert_eq!(tokens.len(), 1, "expected exactly 1 token, got {:?}", tokens);
        assert_matches!(tokens[0], $pat);
    }};
    ($src:expr, $pat:pat if $guard:expr) => {{
        let tokens = tokenize($src);
        assert_eq!(tokens.len(), 1, "expected exactly 1 token, got {:?}", tokens);
        assert_matches!(tokens[0], $pat if $guard);
    }};
}

    #[test]
    fn empty_source_yields_no_tokens() {
        assert!(tokenize("").is_empty());
    }

    #[test]
    fn whitespace_only_yields_no_tokens() {
        assert!(tokenize("   \t\n\r\n   ").is_empty());
    }

    #[test]
    fn comment_alone_yields_no_tokens() {
        assert!(tokenize("# this is a comment").is_empty());
    }

    #[test]
    fn comment_skipped_before_token() {
        single!("# comment\ndef", TokenKind::Def);
    }

    #[test]
    fn comment_skipped_after_token() {
        let tokens = tokenize("def # comment");
        assert_eq!(tokens, vec![TokenKind::Def]);
    }

    #[test]
    fn comment_does_not_consume_newline() {
        // Tokens after the newline following a comment must still be lexed.
        let tokens = tokenize("# line one\ndef foo");
        assert_eq!(tokens.len(), 2);
        assert_matches!(tokens[0], TokenKind::Def);
        assert_matches!(tokens[1], TokenKind::Ident("foo"));
    }

    #[test]
    fn multiple_comments_all_skipped() {
        let tokens = tokenize("# one\n# two\n# three\nextern");
        assert_eq!(tokens, vec![TokenKind::Extern]);
    }

    #[test]
    fn keyword_def() {
        single!("def", TokenKind::Def);
    }

    #[test]
    fn keyword_extern() {
        single!("extern", TokenKind::Extern);
    }

    #[test]
    fn keyword_if() {
        single!("if", TokenKind::If);
    }

    #[test]
    fn keyword_then() {
        single!("then", TokenKind::Then);
    }

    #[test]
    fn keyword_else() {
        single!("else", TokenKind::Else);
    }

    #[test]
    fn keyword_for() {
        single!("for", TokenKind::For);
    }

    #[test]
    fn keyword_in() {
        single!("in", TokenKind::In);
    }

    #[test]
    fn keyword_unary() {
        single!("unary", TokenKind::Unary);
    }

    #[test]
    fn keyword_binary() {
        single!("binary", TokenKind::Binary);
    }

    #[test]
    fn keyword_var() {
        single!("var", TokenKind::Var);
    }

    #[test]
    fn keyword_prefix_is_ident_not_keyword() {
        // "define" starts with "def" but is not the keyword.
        single!("define", TokenKind::Ident("define"));
    }

    #[test]
    fn keyword_suffix_is_ident_not_keyword() {
        // "ndef" ends with "def" but is an identifier.
        single!("ndef", TokenKind::Ident("ndef"));
    }

    #[test]
    fn ident_simple() {
        single!("foo", TokenKind::Ident("foo"));
    }

    #[test]
    fn ident_with_digits() {
        single!("x1", TokenKind::Ident("x1"));
    }

    #[test]
    fn ident_with_underscores() {
        single!("my_var", TokenKind::Ident("my_var"));
    }

    #[test]
    fn ident_leading_underscore() {
        single!("_private", TokenKind::Ident("_private"));
    }

    #[test]
    fn ident_all_caps() {
        single!("FOO", TokenKind::Ident("FOO"));
    }

    #[test]
    fn ident_mixed_case() {
        single!("camelCase", TokenKind::Ident("camelCase"));
    }

    #[test]
    fn ident_does_not_consume_following_op() {
        let tokens = tokenize("x+y");
        assert_eq!(tokens.len(), 3);
        assert_matches!(tokens[0], TokenKind::Ident("x"));
        assert_matches!(tokens[1], TokenKind::Op('+'));
        assert_matches!(tokens[2], TokenKind::Ident("y"));
    }

    #[test]
    fn number_integer() {
        single!("42", TokenKind::Number(42.0));
    }

    #[test]
    fn number_zero() {
        single!("0", TokenKind::Number(0.0));
    }

    #[test]
    #[expect(clippy::approx_constant)]
    fn number_float() {
        single!("3.14", TokenKind::Number(v) if (v - 3.14).abs() < f64::EPSILON);
    }

    #[test]
    fn number_float_leading_zero() {
        single!("0.5", TokenKind::Number(v) if (v - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn number_dot_led_float() {
        // C++ parses ".4" as 0.4; we must match that behaviour.
        single!(".4", TokenKind::Number(v) if (v - 0.4).abs() < f64::EPSILON);
    }

    #[test]
    fn number_dot_led_float_zero() {
        single!(".0", TokenKind::Number(v) if v == 0.0);
    }

    #[test]
    fn number_integer_followed_by_dot_op() {
        // "1." → Number(1.0) then Op('.') — dot is NOT part of the number
        // because no digit follows it.
        let tokens = tokenize("1.");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], TokenKind::Number(1.0));
        assert_eq!(tokens[1], TokenKind::Op('.'));
    }

    #[test]
    fn lone_dot_is_op() {
        // A bare '.' with no trailing digit is an operator, not a number.
        single!(".", TokenKind::Op('.'));
    }

    #[test]
    fn number_does_not_consume_following_op() {
        let tokens = tokenize("4+5");
        assert_eq!(tokens.len(), 3);
        assert_matches!(tokens[0], TokenKind::Number(4.0));
        assert_matches!(tokens[1], TokenKind::Op('+'));
        assert_matches!(tokens[2], TokenKind::Number(5.0));
    }

    #[test]
    fn number_large_integer() {
        single!("9999999", TokenKind::Number(9_999_999.0));
    }

    #[test]
    fn punct_lparen() {
        single!("(", TokenKind::LParen);
    }

    #[test]
    fn punct_rparen() {
        single!(")", TokenKind::RParen);
    }

    #[test]
    fn punct_comma() {
        single!(",", TokenKind::Comma);
    }

    #[test]
    fn op_plus() {
        single!("+", TokenKind::Op('+'));
    }

    #[test]
    fn op_minus() {
        single!("-", TokenKind::Op('-'));
    }

    #[test]
    fn op_star() {
        single!("*", TokenKind::Op('*'));
    }

    #[test]
    fn op_slash() {
        single!("/", TokenKind::Op('/'));
    }

    #[test]
    fn op_lt() {
        single!("<", TokenKind::Op('<'));
    }

    #[test]
    fn op_gt() {
        single!(">", TokenKind::Op('>'));
    }

    #[test]
    fn op_equals() {
        single!("=", TokenKind::Op('='));
    }

    #[test]
    fn op_semicolon() {
        single!(";", TokenKind::Op(';'));
    }

    #[test]
    fn def_with_two_params_and_body() {
        let tokens = tokenize("def foo(x y) x+y");
        assert_eq!(tokens.len(), 9);
        assert_matches!(tokens[0], TokenKind::Def);
        assert_matches!(tokens[1], TokenKind::Ident("foo"));
        assert_matches!(tokens[2], TokenKind::LParen);
        assert_matches!(tokens[3], TokenKind::Ident("x"));
        assert_matches!(tokens[4], TokenKind::Ident("y"));
        assert_matches!(tokens[5], TokenKind::RParen);
        assert_matches!(tokens[6], TokenKind::Ident("x"));
        assert_matches!(tokens[7], TokenKind::Op('+'));
        // 'y' at position 8 follows
    }

    #[test]
    fn def_with_semicolon_and_trailing_expr() {
        // "def foo(x y) x+y y;" — two dispatches from one line.
        let tokens = tokenize("def foo(x y) x+y y;");
        assert_eq!(tokens.len(), 11);
        assert_matches!(tokens[10], TokenKind::Op(';'));
    }

    #[test]
    fn call_with_float_arg() {
        let tokens = tokenize("foo(y, 4.0)");
        assert_eq!(tokens.len(), 6);
        assert_matches!(tokens[0], TokenKind::Ident("foo"));
        assert_matches!(tokens[1], TokenKind::LParen);
        assert_matches!(tokens[2], TokenKind::Ident("y"));
        assert_matches!(tokens[3], TokenKind::Comma);
        assert_matches!(tokens[4], TokenKind::Number(4.0));
        assert_matches!(tokens[5], TokenKind::RParen);
    }

    #[test]
    fn extern_declaration() {
        let tokens = tokenize("extern sin(a);");
        assert_eq!(tokens.len(), 6);
        assert_matches!(tokens[0], TokenKind::Extern);
        assert_matches!(tokens[1], TokenKind::Ident("sin"));
        assert_matches!(tokens[2], TokenKind::LParen);
        assert_matches!(tokens[3], TokenKind::Ident("a"));
        assert_matches!(tokens[4], TokenKind::RParen);
    }

    #[test]
    fn multiline_input() {
        let src = "def fib(x)\n  x + 1";
        let tokens = tokenize(src);
        assert_matches!(tokens[0], TokenKind::Def);
        assert_matches!(tokens[1], TokenKind::Ident("fib"));
        // The newline is just whitespace. no extra tokens emitted.
        assert_matches!(tokens[5], TokenKind::Ident("x"));
        assert_matches!(tokens[6], TokenKind::Op('+'));
        assert_matches!(tokens[7], TokenKind::Number(1.0));
    }

    #[test]
    fn fibonacci_function() {
        let src = "def fib(x) if x < 3 then 1 else fib(x-1)+fib(x-2)";
        let tokens = tokenize(src);
        assert_matches!(tokens[0], TokenKind::Def);
        assert_matches!(tokens[1], TokenKind::Ident("fib"));
        assert_matches!(tokens[5], TokenKind::If);
        assert_matches!(tokens[7], TokenKind::Op('<'));
        assert_matches!(tokens[9], TokenKind::Then);
        assert_matches!(tokens[11], TokenKind::Else);
        assert_matches!(tokens[12], TokenKind::Ident("fib"));
    }

    #[test]
    fn span_single_char_op() {
        let tokens = tokenize_full("+");
        assert_eq!(tokens[0].span, Span { start: 0, end: 1 });
    }

    #[test]
    fn span_keyword() {
        let tokens = tokenize_full("def");
        assert_eq!(tokens[0].span, Span { start: 0, end: 3 });
    }

    #[test]
    fn span_ident() {
        let tokens = tokenize_full("foo");
        assert_eq!(tokens[0].span, Span { start: 0, end: 3 });
    }

    #[test]
    fn span_number() {
        let tokens = tokenize_full("3.14");
        assert_eq!(tokens[0].span, Span { start: 0, end: 4 });
    }

    #[test]
    fn span_accounts_for_leading_whitespace() {
        // "  foo" — foo starts at byte 2.
        let tokens = tokenize_full("  foo");
        assert_eq!(tokens[0].span, Span { start: 2, end: 5 });
    }

    #[test]
    fn span_second_token_correct() {
        // "x+y": '+' is at byte 1.
        let tokens = tokenize_full("x+y");
        assert_eq!(tokens[1].span, Span { start: 1, end: 2 });
    }

    #[test]
    fn span_covers_full_ident() {
        let tokens = tokenize_full("hello");
        assert_eq!(tokens[0].span, Span { start: 0, end: 5 });
        assert_eq!(&"hello"[tokens[0].span.start..tokens[0].span.end], "hello");
    }

    #[test]
    fn iterator_is_fused_after_eof() {
        // A fused iterator must keep returning None after exhaustion.
        let mut lexer = Lexer::new("x");
        assert!(lexer.next().is_some()); // 'x'
        assert!(lexer.next().is_none()); // EOF
        assert!(lexer.next().is_none()); // still None — fused
        assert!(lexer.next().is_none());
    }

    #[test]
    fn tokens_iterator_excludes_comments() {
        // The TokenKinds wrapper must never yield Comment.
        let kinds = Lexer::new("# comment\nfoo # another\nbar").tokens();
        for kind in kinds {
            assert_ne!(kind, TokenKind::Comment);
        }
    }

    #[test]
    fn token_iterator_includes_all_full_tokens() {
        // The Token iterator (with spans) must also skip comments.
        let tokens: Vec<_> = Lexer::new("# comment\nfoo").collect();
        assert_eq!(tokens.len(), 1);
        assert_matches!(tokens[0].kind, TokenKind::Ident("foo"));
    }

    #[test]
    fn cloned_lexer_is_independent() {
        // Lexer is Clone. a clone must not share cursor state.
        let original = Lexer::new("x y");
        let mut clone = original.clone();
        let mut original = original;

        let t1 = original.next().unwrap();
        // Advance original past 'x'. Clone should still start at 'x'.
        let t2 = clone.next().unwrap();
        assert_eq!(t1.kind, t2.kind);
    }

    #[test]
    fn ident_slice_matches_source() {
        let src = "hello";
        let tokens = tokenize_full(src);
        let span = tokens[0].span;
        assert_eq!(&src[span.start..span.end], "hello");
    }

    #[test]
    fn number_slice_matches_source() {
        let src = "3.14";
        let tokens = tokenize_full(src);
        let span = tokens[0].span;
        assert_eq!(&src[span.start..span.end], "3.14");
    }
}
