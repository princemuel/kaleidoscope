//! The Kaleidoscope Parser

use std::collections::HashMap;

use crate::ast::{Expr, Function, Prototype};
use crate::lexer::LexError;
use crate::token::Token;

const ANONYMOUS_FUNCTION: &str = "__anon_expr";

pub struct Parser<'a> {
    tokens: &'a [Token],
    /// The current position of the token the parser is looking at.
    pos: usize,
    /// Holds the precedence for each binary operator.
    prec: &'a mut HashMap<char, u8>,
}

impl<'a> Parser<'a> {
    /// Creates a new parser, given an tokens slice [`Token`] and a [`HashMap`]
    /// binding an operator and its precedence in binary expressions.
    pub fn new(tokens: &'a [Token], prec: &'a mut HashMap<char, u8>) -> Self {
        Self {
            tokens,
            prec,
            pos: 0,
        }
    }

    /// Parses the content of the parser.
    pub fn parse(&mut self) -> ParseResult<Function> {
        let result = match self.current()? {
            Token::Def => self.parse_definition(),
            Token::Extern => self.parse_extern(),
            _ => self.parse_toplevel_expr(),
        }?;

        if self.is_eof() {
            Ok(result)
        } else {
            Err(ParseError::TrailingTokens(self.current()?))
        }
    }

    /// Returns the current [`Token`], or
    /// [`UnexpectedEof`](`ParseError::UnexpectedEof`) if the parser has
    /// reached the end of the token stream.
    pub fn current(&self) -> ParseResult<Token> {
        self.tokens
            .get(self.pos)
            .cloned()
            .ok_or(ParseError::UnexpectedEof)
    }

    /// Advances the position by one. Returns
    /// [`UnexpectedEof`](`ParseError::UnexpectedEof`) if already at the end
    /// of the token stream, allowing `self.advance()?`.
    pub fn advance(&mut self) -> ParseResult<()> {
        self.pos += 1;

        if self.is_eof() {
            Err(ParseError::UnexpectedEof)
        } else {
            Ok(())
        }
    }

    /// Returns `true` if the [`Parser`] has reached the end of the token
    /// stream.
    #[must_use]
    pub const fn is_eof(&self) -> bool { self.pos >= self.tokens.len() }

    /// Returns the precedence of the current `Token`, or `None` if it is not
    /// a known binary operator.
    #[must_use]
    pub fn tok_precedence(&self) -> Option<u8> {
        let Token::Op(op) = self.current().ok()? else {
            return None;
        };

        let prec = self.prec.get(&op).copied()?;
        (prec > 0).then_some(prec)
    }

    /// Parses any expression.
    ///
    /// expression ::= primary binoprhs
    pub fn parse_expr(&mut self) -> ParseResult<Expr> {
        let lhs = self.parse_unary_expr()?;
        self.parse_bin_expr(0, lhs)
    }

    /// Parses a literal number.
    ///
    /// numberexpr ::= number
    pub fn parse_num_expr(&mut self) -> ParseResult<Expr> {
        let token = self.current()?;
        let Token::Number(result) = token else {
            return Err(ParseError::ExpectedNumber(token));
        };

        self.advance().ok();
        Ok(Expr::Number(result))
    }

    /// Parses an expression enclosed in parentheses.
    ///
    /// parenexpr ::= '(' expression ')'
    pub fn parse_paren_expr(&mut self) -> ParseResult<Expr> {
        let token = self.current()?;
        let Token::LParen = token else {
            return Err(ParseError::ExpectedLParen(token));
        };

        self.advance()?;
        let result = self.parse_expr()?;

        let token = self.current()?;
        let Token::RParen = token else {
            return Err(ParseError::ExpectedRParen(token));
        };

        self.advance().ok();
        Ok(result)
    }

    /// Parses an expression that starts with an identifier (either a variable
    /// or a function call).
    ///
    /// identifierexpr ::= identifier
    ///                  | identifier '(' expression* ')'
    pub fn parse_ident_expr(&mut self) -> ParseResult<Expr> {
        let token = self.current()?;
        let Token::Ident(ident) = token else {
            return Err(ParseError::ExpectedIdent(token));
        };

        // Not a call — either EOF or a non-'(' token follows.
        if self.advance().is_err() || !matches!(self.current()?, Token::LParen) {
            return Ok(Expr::Variable(ident));
        }

        // Consume past '('
        self.advance()?;

        let mut args = Vec::new();
        // Handles both the no-arg case (immediate ')') and the multi-arg case.
        while !matches!(self.current()?, Token::RParen) {
            args.push(self.parse_expr()?);

            match self.current()? {
                Token::Comma => self.advance()?,
                Token::RParen => break,
                token => return Err(ParseError::ExpectedCommaOrRParen(token)),
            }
        }

        self.advance().ok(); // consume ')', soft-fail at EOF
        Ok(Expr::Call { name: ident, args })
    }

    /// Parses a primary expression (identifier, number, or parenthesized).
    ///
    /// primary ::= identifierexpr | numberexpr | parenexpr
    pub fn parse_primary(&mut self) -> ParseResult<Expr> {
        match self.current()? {
            Token::Ident(_) => self.parse_ident_expr(),
            Token::Number(_) => self.parse_num_expr(),
            Token::LParen => self.parse_paren_expr(),
            token => Err(ParseError::ExpectedExpr(token)),
        }
    }

    /// Parses a unary expression.
    ///
    /// unary ::= op unary | primary
    pub fn parse_unary_expr(&mut self) -> ParseResult<Expr> {
        if let Token::Op(op) = self.current()? {
            self.advance()?;

            return Ok(Expr::Call {
                name: format!("unary{op}"),
                args: vec![self.parse_unary_expr()?],
            });
        }
        self.parse_primary()
    }

    /// Parses a binary expression given its left-hand side.
    ///
    /// binoprhs ::= (op unary)*
    pub fn parse_bin_expr(&mut self, prec: u8, mut lhs: Expr) -> ParseResult<Expr> {
        loop {
            let tok_prec = match self.tok_precedence() {
                Some(p) if p >= prec => p,
                _ => return Ok(lhs),
            };

            let token = self.current()?;
            let Token::Op(op) = token else {
                return Err(ParseError::InvalidOperator(token));
            };

            self.advance()?;

            let mut rhs = self.parse_unary_expr()?;
            if let Some(next_prec) = self.tok_precedence()
                && tok_prec < next_prec
            {
                rhs = self.parse_bin_expr(tok_prec + 1, rhs)?;
            }

            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
    }

    /// Parses a function prototype.
    ///
    /// prototype ::= id '(' id* ')'
    ///             | 'binary' op number? '(' id id ')'
    ///             | 'unary'  op         '(' id ')'
    pub fn parse_prototype(&mut self) -> ParseResult<Prototype> {
        let (ident, is_op, prec) = match self.current()? {
            Token::Ident(ident) => {
                self.advance()?;
                (ident, false, 0)
            }

            Token::Binary => {
                self.advance()?;

                let token = self.current()?;
                let Token::Op(op) = token else {
                    return Err(ParseError::ExpectedOperator(token));
                };

                self.advance()?;

                // * manual check for safe `as` conversion from double (f64) value
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss
                )]
                let prec = if let Ok(Token::Number(n)) = self.current() {
                    self.advance()?;
                    if n.is_finite() { n as u8 } else { 0 }
                } else {
                    0
                };

                self.prec.insert(op, prec);
                (format!("binary{op}"), true, prec)
            }

            Token::Unary => {
                self.advance()?;

                let token = self.current()?;
                let Token::Op(op) = token else {
                    return Err(ParseError::ExpectedOperator(token));
                };

                self.advance()?;

                (format!("unary{op}"), true, 0)
            }

            token => return Err(ParseError::ExpectedPrototypeName(token)),
        };

        let token = self.current()?;
        let Token::LParen = token else {
            return Err(ParseError::ExpectedLParen(token));
        };

        let mut args = Vec::new();
        // Handles both the no-arg case (immediate ')') and the multi-arg case.
        while !matches!(self.current()?, Token::RParen) {
            let token = self.current()?;
            let Token::Ident(name) = token else {
                return Err(ParseError::ExpectedIdent(token));
            };

            args.push(name);
            self.advance()?;

            match self.current()? {
                Token::Comma => self.advance()?,
                Token::RParen => break,
                token => return Err(ParseError::ExpectedCommaOrRParen(token)),
            }
        }

        self.advance().ok(); // consume RParen, soft-fail at EOF

        Ok(Prototype {
            name: ident,
            args,
            is_op,
            prec,
        })
    }

    /// Parses a function definition.
    ///
    /// definition ::= 'def' prototype expression
    pub fn parse_definition(&mut self) -> ParseResult<Function> {
        self.advance().ok(); // eat 'def'

        let proto = self.parse_prototype()?;
        let body = self.parse_expr()?;

        Ok(Function {
            proto,
            body: Some(body),
            is_anon: false,
        })
    }

    /// Parses an external function declaration.
    ///
    /// external ::= 'extern' prototype
    pub fn parse_extern(&mut self) -> ParseResult<Function> {
        self.advance().ok(); // eat 'extern'
        let proto = self.parse_prototype()?;

        Ok(Function {
            proto,
            body: None,
            is_anon: false,
        })
    }

    /// Parses a top-level expression as an anonymous function.
    ///
    /// toplevelexpr ::= expression
    pub fn parse_toplevel_expr(&mut self) -> ParseResult<Function> {
        let expr = self.parse_expr()?;
        Ok(Function {
            proto: Prototype {
                name: ANONYMOUS_FUNCTION.to_owned(),
                args: vec![],
                prec: 0,
                is_op: false,
            },
            body: Some(expr),
            is_anon: true,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("unexpected end of input")]
    UnexpectedEof,

    #[error("expected number literal, got {0:?}")]
    ExpectedNumber(Token),

    #[error("expected identifier, got {0:?}")]
    ExpectedIdent(Token),

    #[error("expected '(' got {0:?}")]
    ExpectedLParen(Token),

    #[error("expected ')' got {0:?}")]
    ExpectedRParen(Token),

    #[error("expected ',' or ')' in argument list, got {0:?}")]
    ExpectedCommaOrRParen(Token),

    #[error("expected operator in custom operator declaration, got {0:?}")]
    ExpectedOperator(Token),

    #[error("expected identifier or 'binary' in prototype, got {0:?}")]
    ExpectedPrototypeName(Token),

    #[error("expected expression, got {0:?}")]
    ExpectedExpr(Token),

    #[error("invalid binary operator: {0:?}")]
    InvalidOperator(Token),

    #[error("invalid operator precedence: {0:?}")]
    InvalidPrecedence(f64),

    #[error("unexpected tokens after parsed expression, starting at {0:?}")]
    TrailingTokens(Token),

    #[error("lex error: {0}")]
    Lex(#[from] LexError),
}

pub type ParseResult<T> = Result<T, ParseError>;
