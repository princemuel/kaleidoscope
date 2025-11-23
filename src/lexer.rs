//! The Kaleidoscope Lexer

use core::iter::Peekable;
use core::str::Chars;
use std::io;

use crate::token::Token;

pub struct Lexer<'a> {
    pos:   usize,
    input: &'a str,
    chars: Box<Peekable<Chars<'a>>>,
}

impl<'a> Lexer<'a> {
    /// Creates a new `Lexer`,
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: Box::new(input.chars().peekable()),
            pos: 0,
        }
    }

    pub fn token(&mut self) -> io::Result<Token> {
        self.skip_whitespace();

        let start = self.pos;

        // Check for end of file. Don't eat the EOF.
        let &ch = match self.chars.peek() {
            Some(c) => c,
            None => return Ok(Token::EOF),
        };

        self.advance();

        let token = match ch {
            '(' => Token::LParen,
            ')' => Token::RParen,
            ',' => Token::Comma,
            '#' => self.lex_comment(),
            '.' | '0'..='9' => self.lex_number(start),
            'a'..='z' | 'A'..='Z' | '_' => self.lex_ident(start),
            op => Token::Op(op),
        };

        Ok(token)
    }

    #[inline]
    fn advance(&mut self) {
        self.chars.next();
        self.pos += 1;
    }

    fn skip_whitespace(&mut self) {
        while let Some(&ch) = self.chars.peek() {
            if !ch.is_whitespace() {
                break;
            }
            self.advance();
        }
    }

    fn lex_comment(&mut self) -> Token {
        while let Some(&ch) = self.chars.peek() {
            self.advance();
            if ch == '\n' || ch == '\r' {
                break;
            }
        }
        Token::Comment
    }

    fn lex_number(&mut self, start: usize) -> Token {
        while let Some(&ch) = self.chars.peek() {
            if ch != '.' && !ch.is_ascii_hexdigit() {
                break;
            }
            self.advance();
        }

        let slice = &self.input[start..self.pos];
        Token::Number(slice.parse().unwrap_or_default())
    }

    fn lex_ident(&mut self, start: usize) -> Token {
        while let Some(&ch) = self.chars.peek() {
            if ch != '_' && !ch.is_alphanumeric() {
                break;
            }
            self.advance();
        }

        match &self.input[start..self.pos] {
            "def" => Token::Def,
            "extern" => Token::Extern,
            "binary" => Token::Binary,
            ident => Token::Ident(ident.to_string()),
        }
    }
}

impl Iterator for Lexer<'_> {
    type Item = Token;

    /// Lexes the next `Token` and returns it. `None` is returned on EOF or
    /// failure
    fn next(&mut self) -> Option<Self::Item> {
        match self.token() {
            Ok(Token::EOF) | Err(_) => None,
            Ok(value) => Some(value),
        }
    }
}
