use std::collections::HashMap;

use crate::ast::Expr;
use crate::error::ParseError;
use crate::token::TokenKind;

const FUNCTION: &str = "anonymous";

#[derive(Clone, Debug)]
pub struct Parser<'a> {
    tokens: &'a [TokenKind<'a>],
    /// The current position of the token the parser is looking at.
    cursor: usize,
    /// Holds the precedence for each binary operator.
    prec: &'a HashMap<char, u8>,
}

impl<'a> Parser<'a> {
    /// Creates a new parser, given an token slice, and a hashmap
    /// binding an operator and its precedence in binary expressions.
    pub const fn new(tokens: &'a [TokenKind<'_>], prec: &'a mut HashMap<char, u8>) -> Self {
        Self { tokens, prec, cursor: 0 }
    }

    /// Parses any expression.
    ///
    /// expression ::= primary binoprhs
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> { unimplemented!() }

    /// Parses a literal number.
    ///
    /// numberexpr ::= number
    pub fn parse_num_expr(&mut self) -> Result<Expr, ParseError> {
        let token = self.current()?;

        let TokenKind::Number(result) = token else {
            return Err(ParseError::ExpectedNumber(token.to_string()));
        };

        self.advance().ok();
        Ok(Expr::Number(result))
    }

    /// Parses an expression enclosed in parentheses.
    ///
    /// parenexpr ::= '(' expression ')'
    pub fn parse_paren_expr(&mut self) -> Result<Expr, ParseError> {
        let token = self.current()?;
        let TokenKind::LParen = token else {
            return Err(ParseError::ExpectedLParen(token.to_string()));
        };
        self.advance()?;

        let result = self.parse_expr()?;

        let token = self.current()?;
        let TokenKind::LParen = token else {
            return Err(ParseError::ExpectedRParen(token.to_string()));
        };
        self.advance().ok();

        Ok(result)
    }

    /// Parses an expression that starts with an identifier (either a variable
    /// or a function call).
    ///
    /// identifierexpr ::= identifier
    ///                  | identifier '(' expression* ')'
    pub fn parse_ident_expr(&mut self) -> Result<Expr, ParseError> {
        let token = self.current()?;
        let TokenKind::Ident(ident) = token else {
            return Err(ParseError::ExpectedIdent(token.to_string()));
        };

        let ident = ident.to_owned();

        // Not a call. either EOF or a non-'(' token follows.
        if self.advance().is_err() || !matches!(self.current()?, TokenKind::LParen) {
            return Ok(Expr::Variable(ident.clone()));
        }

        // Consume past '('
        self.advance()?;

        let mut args = Vec::new();
        // Handles both the no-arg case (immediate ')') and the multi-arg case.
        while !matches!(self.current()?, TokenKind::RParen) {
            args.push(self.parse_expr()?);

            match self.current()? {
                TokenKind::Comma => self.advance()?,
                TokenKind::RParen => break,
                t => {
                    return Err(ParseError::ExpectedCommaOrRParen(t.to_string()));
                }
            }
        }

        self.advance().ok(); // consume ')', soft-fail at EOF
        Ok(Expr::Call { name: ident.clone(), args })
    }

    /// Parses a primary expression (identifier, number, or parenthesized).
    ///
    /// primary ::= identifierexpr | numberexpr | parenexpr
    pub fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.current()? {
            TokenKind::Ident(_) => self.parse_ident_expr(),
            TokenKind::Number(_) => self.parse_num_expr(),
            TokenKind::LParen => self.parse_paren_expr(),
            token => Err(ParseError::ExpectedExpr(token.to_string())),
        }
    }
}

impl Parser<'_> {
    /// Returns the current [`TokenKind`], or
    /// [`UnexpectedEof`](`ParseError::UnexpectedEof`) if the parser has
    /// reached the end of the token stream.
    pub fn current(&self) -> Result<TokenKind<'_>, ParseError> {
        self.tokens.get(self.cursor).copied().ok_or(ParseError::UnexpectedEof)
    }

    pub fn advance(&mut self) -> Result<(), ParseError> {
        self.cursor += 1;
        if self.is_eof() { Err(ParseError::UnexpectedEof) } else { Ok(()) }
    }

    /// Returns the precedence of the current [`TokenKind`], or `None` if it is
    /// not a known binary operator.
    #[must_use]
    pub fn tok_precedence(&self) -> Option<u8> {
        let TokenKind::Op(op) = self.current().ok()? else { return None };

        let precedence = self.prec.get(&op).copied()?;
        (precedence > 0).then_some(precedence)
    }

    /// Returns `true` if [`Parser`] has reached the end of the token stream.
    #[must_use]
    pub const fn is_eof(&self) -> bool { self.cursor >= self.tokens.len() }
}
