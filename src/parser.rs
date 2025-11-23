//! The Kaleidoscope Parser

use std::collections::HashMap;
use std::io;

use crate::ast::{Expr, Function, Prototype};
use crate::lexer::Lexer;
use crate::token::Token;

enum PE {
    Syntax,
    Eof,
}
const FUNC_NAME: &str = "anon";

pub struct Parser<'a> {
    tokens: Vec<Token>,
    /// The current position of the token the parser is looking at.
    pos:    usize,
    /// Holds the precedence for each binary operator.
    prec:   &'a mut HashMap<char, i32>,
}

impl<'a> Parser<'a> {
    pub fn new(input: impl AsRef<str>, prec: &'a mut HashMap<char, i32>) -> Self {
        let mut lexer = Lexer::new(input.as_ref());
        let tokens = lexer.by_ref().collect();

        Self { tokens, prec, pos: 0 }
    }

    /// Parses the content of the parser.
    pub fn parse(&mut self) -> io::Result<Function> {
        let result = match self.current()? {
            Token::Def => self.parse_definition(),
            Token::Extern => self.parse_extern(),
            _ => self.parse_toplevel_expr(),
        };

        match result {
            Ok(result) => {
                if !self.is_eof() {
                    Err(self.log_err(PE::Eof, "Unexpected token after parsed expression."))
                } else {
                    Ok(result)
                }
            },
            err => err,
        }
    }

    /// Returns the current `Token`, or an error that
    /// indicates that the end of the file has been unexpectedly reached
    pub fn current(&self) -> io::Result<Token> {
        if self.is_eof() {
            Err(self.log_err(PE::Eof, "Unexpected end of file."))
        } else {
            Ok(self.tokens[self.pos].clone())
        }
    }

    /// Advances the position, and returns an empty `Result` whose error
    /// indicates that the end of the file has been unexpectedly reached.
    /// This allows to use the `self.advance()?;` syntax.
    pub fn advance(&mut self) -> io::Result<()> {
        self.pos += 1;

        (!self.is_eof())
            .then_some(())
            .ok_or_else(|| self.log_err(PE::Eof, "Unexpected end of file."))
    }

    /// Returns a value indicating whether or not the `Parser`
    /// has reached the end of the input.
    pub const fn is_eof(&self) -> bool { self.pos >= self.tokens.len() }

    /// Returns the precedence of the current `Token`, or -1 if it is not
    /// recognized as a binary operator.
    pub fn tok_precedence(&self) -> i32 {
        match self.current() {
            Ok(Token::Op(op)) => self.prec.get(&op).copied().unwrap_or(-1),
            _ => -1,
        }
    }

    /// Parses any expression.
    ///
    /// expression ::= primary binoprhs
    pub fn parse_expr(&mut self) -> io::Result<Expr> {
        match self.parse_unary_expr() {
            Ok(lhs) => self.parse_bin_expr(0, lhs),
            err => err,
        }
    }

    /// Parses a literal number.
    ///
    /// numberexpr ::= number
    pub fn parse_num_expr(&mut self) -> io::Result<Expr> {
        if let Token::Number(value) = self.current()? {
            self.advance()?;
            Ok(Expr::Number(value))
        } else {
            Err(self.log_err(PE::Syntax, "expected number literal."))
        }
    }

    /// Parses an expression enclosed in parenthesis.
    ///
    /// parenexpr ::= '(' expression ')'
    pub fn parse_paren_expr(&mut self) -> io::Result<Expr> {
        match self.current()? {
            Token::LParen => (),
            _ => {
                return Err(self.log_err(
                    PE::Syntax,
                    "Expected '(' character at start of parenthesized expression.",
                ));
            },
        }

        self.advance()?;

        let expr = self.parse_expr()?;

        match self.current()? {
            Token::RParen => (),
            _ => {
                return Err(self.log_err(
                    PE::Syntax,
                    "Expected ')' character at end of parenthesized expression.",
                ));
            },
        }

        self.advance()?;

        Ok(expr)
    }

    /// Parses an expression that starts with an identifier (either a variable
    /// or a function call).
    ///
    /// identifierexpr ::= identifier ::= identifier '(' expression* ')'
    pub fn parse_ident_expr(&mut self) -> io::Result<Expr> {
        let ident = if let Token::Ident(id) = &self.current()? {
            id.clone()
        } else {
            return Err(self.log_err(PE::Syntax, "Expected identifier"));
        };

        // Simple variable ref
        if self.advance().is_err() {
            return Ok(Expr::Variable(ident));
        }

        match self.current()? {
            Token::LParen => {
                self.advance()?;
                if let Token::RParen = self.current()? {
                    return Ok(Expr::Call {
                        name: ident,
                        args: vec![],
                    });
                }

                let mut args = vec![];

                loop {
                    args.push(self.parse_expr()?);

                    match self.current()? {
                        Token::Comma => (),
                        Token::RParen => break,
                        _ => {
                            return Err(
                                self.log_err(PE::Syntax, "Expected ',' character in function call.")
                            );
                        },
                    }

                    self.advance()?;
                }

                self.advance()?;

                Ok(Expr::Call { name: ident, args })
            },

            _ => Ok(Expr::Variable(ident)),
        }
    }

    /// Parses a primary expression (an identifier, a number or a parenthesized
    /// expression).
    ///
    /// primary ::= identifierexpr ::= numberexpr ::= parenexpr
    pub fn parse_primary(&mut self) -> io::Result<Expr> {
        match self.current()? {
            Token::Ident(_) => self.parse_ident_expr(),
            Token::Number(_) => self.parse_num_expr(),
            Token::LParen => self.parse_paren_expr(),
            _ => Err(self.log_err(PE::Syntax, "unknown token when expecting an expression")),
        }
    }

    /// Parses an unary expression.
    pub fn parse_unary_expr(&mut self) -> io::Result<Expr> {
        match self.current()? {
            Token::Op(op) => {
                self.advance()?;

                let name = format!("unary{op}");
                Ok(Expr::Call {
                    name,
                    args: vec![self.parse_unary_expr()?],
                })
            },
            _ => self.parse_primary(),
        }
    }

    /// Parses a binary expression, given its left-hand expression.
    pub fn parse_bin_expr(&mut self, prec: i32, mut lhs: Expr) -> io::Result<Expr> {
        loop {
            let curr_prec = self.tok_precedence();
            if curr_prec < prec || self.is_eof() {
                return Ok(lhs);
            }

            let op = match self.current()? {
                Token::Op(op) => op,
                _ => return Err(self.log_err(PE::Syntax, "Invalid operator.")),
            };

            self.advance()?;

            // If BinOp binds less tightly with RHS than the operator after RHS, let
            // the pending operator take RHS as its LHS.
            let mut rhs = self.parse_unary_expr()?;
            let next_prec = self.tok_precedence();

            if curr_prec < next_prec {
                rhs = self.parse_bin_expr(curr_prec + 1, rhs)?;
            }

            // Merge LHS/RHS.
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
    }

    /// prototype  ::= id '(' id* ')'
    pub fn parse_prototype(&mut self) -> io::Result<Prototype> {
        let (id, is_operator, precedence) = match self.current()? {
            Token::Ident(id) => {
                self.advance()?;
                (id, false, 0)
            },

            Token::Binary => {
                self.advance()?;

                let op = match self.current()? {
                    Token::Op(ch) => ch,
                    _ => {
                        return Err(
                            self.log_err(PE::Syntax, "Expected operator in custom operator declaration.")
                        );
                    },
                };

                self.advance()?;

                let name = format!("binary{op}");

                let prec = if let Token::Number(prec) = self.current()? {
                    self.advance()?;
                    prec as usize
                } else {
                    0
                };

                self.prec.insert(op, prec as i32);

                (name, true, prec)
            },

            _ => return Err(self.log_err(PE::Syntax, "Expected identifier in prototype declaration.")),
        };

        match self.current()? {
            Token::LParen => (),
            _ => return Err(self.log_err(PE::Syntax, "Expected '(' character in prototype declaration.")),
        }

        self.advance()?;

        if let Token::RParen = self.current()? {
            self.advance()?;

            return Ok(Prototype {
                name:  id,
                args:  vec![],
                is_op: is_operator,
                prec:  precedence,
            });
        }

        let mut args = vec![];

        loop {
            match self.current()? {
                Token::Ident(name) => args.push(name),
                _ => return Err(self.log_err(PE::Syntax, "Expected identifier in parameter declaration.")),
            }

            self.advance()?;

            match self.current()? {
                Token::RParen => {
                    let _ = self.advance();
                    break;
                },
                Token::Comma => {
                    let _ = self.advance();
                },
                _ => {
                    return Err(self.log_err(
                        PE::Syntax,
                        "Expected ',' or ')' character in prototype declaration.",
                    ));
                },
            }
        }

        Ok(Prototype {
            name: id,
            args,
            is_op: is_operator,
            prec: precedence,
        })
    }

    /// definition ::= 'def' prototype expression
    pub fn parse_definition(&mut self) -> io::Result<Function> {
        // Eat 'def' keyword
        self.pos += 1;

        // Parse signature of function
        let proto = self.parse_prototype()?;

        // Parse function body
        let body = self.parse_expr()?;

        // Return new function
        Ok(Function {
            proto,
            body: Some(body),
            is_anon: false,
        })
    }

    /// Parses an external function declaration.
    ///
    /// external ::= 'extern' prototype
    pub fn parse_extern(&mut self) -> io::Result<Function> {
        // Eat 'extern' keyword
        self.pos += 1;

        // Parse signature of extern function
        let proto = self.parse_prototype()?;
        Ok(Function {
            proto,
            body: None,
            is_anon: false,
        })
    }

    /// toplevelexpr ::= expression
    pub fn parse_toplevel_expr(&mut self) -> io::Result<Function> {
        match self.parse_expr() {
            Ok(value) => Ok(Function {
                proto:   Prototype {
                    name:  FUNC_NAME.to_string(),
                    args:  vec![],
                    prec:  0,
                    is_op: false,
                },
                body:    Some(value),
                is_anon: true,
            }),
            Err(value) => Err(value),
        }
    }

    fn log_err(&self, kind: PE, error: &str) -> io::Error {
        let kind = match kind {
            PE::Syntax => io::ErrorKind::InvalidData,
            PE::Eof => io::ErrorKind::UnexpectedEof,
        };
        io::Error::new(kind, error)
    }
}
