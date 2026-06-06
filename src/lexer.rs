use crate::token::{Span, Token};

#[derive(Clone, Debug)]
pub(crate) struct Lexer<'a> {
    input: &'a str,
    /// The current position of the token
    cursor: usize,
}

impl<'a> Lexer<'a> {
    pub(crate) const fn new(input: &'a str) -> Self { Self { input, cursor: 0 } }

    fn next_token(&mut self) -> Option<Token<'a>> {
        // skip whitespace
        self.advance_while(|b| b.is_ascii_whitespace());

        todo!()
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

    fn span(&self, start: usize) -> Span { Span { start, end: self.cursor } }

    fn slice(&self, start: usize) -> &'a str { &self.input[start..self.cursor] }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> { self.next_token() }
}

mod models {}
