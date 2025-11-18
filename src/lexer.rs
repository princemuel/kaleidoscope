use std::io::BufRead;

use crate::token::Token;

pub struct Lexer<R: BufRead> {
    input:     R,
    last_char: Option<char>,
}

impl<R: BufRead> Lexer<R> {
    pub const fn new(input: R) -> Self {
        Self {
            input,
            last_char: Some(' '),
        }
    }

    pub fn get_token(&mut self) -> Token {
        // Skip any whitespace
        while self.last_char.is_some_and(char::is_whitespace) {
            self.last_char = self.get_char();
        }

        // Check for end of file. Don't eat the EOF.
        let current = match self.last_char {
            Some(c) => c,
            None => return Token::Eof,
        };

        // identifier: [a-zA-Z][a-zA-Z0-9]*
        if current.is_alphabetic() {
            let mut value = String::from(current);
            loop {
                self.last_char = self.get_char();
                match self.last_char {
                    Some(ch) if ch.is_alphanumeric() => value.push(ch),
                    _ => break,
                }
            }

            return match value.as_str() {
                "def" => Token::Def,
                "extern" => Token::Extern,
                _ => Token::Identifier(value),
            };
        }

        // Number: [0-9.]+
        // ! NOTE: this isn’t doing sufficient error checking:
        // ! It will incorrectly read “1.23.45.67” and assume it is “1.23”.
        if current.is_ascii_digit() || current == '.' {
            let mut value = String::from(current);
            loop {
                self.last_char = self.get_char();
                match self.last_char {
                    Some(ch) if ch.is_ascii_digit() || ch == '.' => value.push(ch),
                    _ => break,
                }
            }

            return Token::Number(value.parse().unwrap_or_default());
        }

        // Comment until end of line
        if current == '#' {
            loop {
                self.last_char = self.get_char();
                match self.last_char {
                    None | Some('\n') | Some('\r') => break,
                    _ => continue,
                }
            }

            if self.last_char.is_some() {
                return self.get_token();
            }
        }

        // Otherwise, just return the character as its ascii value.
        let this_char = current;
        self.last_char = self.get_char();
        Token::Char(this_char)
    }

    fn get_char(&mut self) -> Option<char> {
        let mut buffer = [0; 1];

        match self.input.read_exact(&mut buffer) {
            Ok(_) => Some(buffer[0] as char),
            Err(_) => None,
        }
    }
}
