use std::collections::HashMap;

use crate::ast::{Expr, Function, Prototype};
use crate::error::ParseError;
use crate::token::TokenKind;

/// the synthetic prototype wrapping a top-level expression.
const ANON_FN: &str = "__anon_expr";

/// A recursive-descent / Pratt parser for the Kaleidoscope language
#[derive(Clone, Debug)]
pub struct Parser<'a> {
    tokens: &'a [TokenKind<'a>],
    /// Index of the token currently being examined.
    cursor: usize,
    /// Binary-operator precedence table. Keyed by operator character.
    prec: &'a HashMap<char, u8>,
}

impl<'a> Parser<'a> {
    /// Construct a parser from a pre-tokenised slice and a precedence table.
    #[must_use]
    pub const fn new(tokens: &'a [TokenKind<'a>], prec: &'a HashMap<char, u8>) -> Self {
        Self { tokens, prec, cursor: 0 }
    }

    /// expression ::= primary binoprhs
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.parse_primary()?;
        self.parse_bin_expr(0, lhs)
    }

    /// numberexpr ::= number
    pub fn parse_num_expr(&mut self) -> Result<Expr, ParseError> {
        let token = self.current()?;
        let TokenKind::Number(n) = token else {
            return Err(ParseError::ExpectedNumber(token.to_string()));
        };

        self.advance_unchecked(); // consume the number; soft-fail at EOF is fine here
        Ok(Expr::Number(n))
    }

    /// parenexpr ::= '(' expression ')'
    pub fn parse_paren_expr(&mut self) -> Result<Expr, ParseError> {
        // Validate and consume '('
        let token = self.current()?;
        let TokenKind::LParen = token else {
            return Err(ParseError::ExpectedLParen(token.to_string()));
        };

        self.advance()?; // must have tokens after '('

        let result = self.parse_expr()?;

        // Validate and consume ')'
        let token = self.current()?;
        let TokenKind::RParen = token else {
            return Err(ParseError::ExpectedRParen(token.to_string()));
        };

        self.advance_unchecked(); // ')' may be the last token
        Ok(result)
    }

    /// identifierexpr ::= identifier
    ///                   | identifier '(' expression* ')'
    pub fn parse_ident_expr(&mut self) -> Result<Expr, ParseError> {
        let token = self.current()?;
        let TokenKind::Ident(ident) = token else {
            return Err(ParseError::ExpectedIdent(token.to_string()));
        };
        let name = ident.to_owned();

        self.advance_unchecked(); // move past identifier; may hit EOF

        if !matches!(self.current(), Ok(TokenKind::LParen)) {
            return Ok(Expr::Variable(name));
        }

        // It's a call. consume '('
        self.advance()?; // must have tokens inside arg list or ')'

        let mut args = Vec::new();
        while !matches!(self.current()?, TokenKind::RParen) {
            args.push(self.parse_expr()?);

            match self.current()? {
                TokenKind::Comma => self.advance()?, // consume ',' then expect more
                TokenKind::RParen => break,
                t => return Err(ParseError::ExpectedCommaOrRParen(t.to_string())),
            }
        }

        self.advance_unchecked(); // consume ')'; soft-fail at EOF
        Ok(Expr::Call { name, args })
    }

    /// primary ::= identifierexpr | numberexpr | parenexpr
    pub fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.current()? {
            TokenKind::Ident(_) => self.parse_ident_expr(),
            TokenKind::Number(_) => self.parse_num_expr(),
            TokenKind::LParen => self.parse_paren_expr(),
            token => Err(ParseError::ExpectedExpr(token.to_string())),
        }
    }

    /// binoprhs ::= (op primary)*
    ///
    /// Pratt / precedence-climbing parser. `min_prec` is the minimum
    /// precedence level the next operator must meet to be consumed.
    pub fn parse_bin_expr(&mut self, min_prec: u8, mut lhs: Expr) -> Result<Expr, ParseError> {
        loop {
            // If the current token is not a known operator, or its precedence
            // is below `min_prec`, we are done climbing.
            let tok_prec = match self.tok_precedence() {
                Some(p) if p >= min_prec => p,
                _ => return Ok(lhs),
            };

            let token = self.current()?;
            let TokenKind::Op(op) = token else {
                // tok_precedence confirmed it's an Op, so this branch is
                // unreachable but we handle it for exhaustiveness.
                return Err(ParseError::InvalidOperator(token.to_string()));
            };

            // Consume the operator. Hard-fail: there must be a RHS.
            self.advance()?;

            let mut rhs = self.parse_primary()?;

            // If the next operator binds more tightly, give it the RHS first.
            if let Some(next_prec) = self.tok_precedence()
                && tok_prec < next_prec
            {
                // used saturating add in the case where there's a user-defined operator with
                // precedence 255, to avoid overflow.
                rhs = self.parse_bin_expr(tok_prec.saturating_add(1), rhs)?;
            }

            lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
    }

    /// prototype ::= id '(' id* ')'
    pub fn parse_prototype(&mut self) -> Result<Prototype, ParseError> {
        let token = self.current()?;
        let TokenKind::Ident(ident) = token else {
            return Err(ParseError::ExpectedPrototypeName(token.to_string()));
        };
        let name = ident.to_owned();
        self.advance()?;

        let token = self.current()?;
        let TokenKind::LParen = token else {
            return Err(ParseError::ExpectedLParen(token.to_string()));
        };
        self.advance()?;

        let mut args = Vec::new();
        while let Ok(TokenKind::Ident(arg)) = self.current() {
            args.push(arg.to_owned());
            self.advance_unchecked(); // EOF after last arg is acceptable
        }

        let token = self.current()?;
        let TokenKind::RParen = token else {
            return Err(ParseError::ExpectedRParen(token.to_string()));
        };

        self.advance_unchecked(); // ')' may be the last token
        Ok(Prototype { name, args })
    }

    /// definition ::= 'def' prototype expression
    pub fn parse_definition(&mut self) -> Result<Function, ParseError> {
        self.advance()?; // eat 'def'
        let proto = self.parse_prototype()?;
        let body = self.parse_expr()?;
        Ok(Function { proto, body: Some(body) })
    }

    /// external ::= 'extern' prototype
    pub fn parse_extern(&mut self) -> Result<Function, ParseError> {
        self.advance()?; // eat 'extern'
        let proto = self.parse_prototype()?;
        Ok(Function { proto, body: None })
    }

    /// toplevelexpr ::= expression
    ///
    /// Wraps a bare expression in a synthetic anonymous function
    pub fn parse_toplevel_expr(&mut self) -> Result<Function, ParseError> {
        let expr = self.parse_expr()?;
        Ok(Function {
            proto: Prototype { name: ANON_FN.to_owned(), args: vec![] },
            body: Some(expr),
        })
    }
}

impl<'a> Parser<'a> {
    /// Returns the current token, or `EOF` if the cursor is past
    /// the end of the token slice.
    pub fn current(&self) -> Result<TokenKind<'a>, ParseError> {
        self.tokens.get(self.cursor).copied().ok_or(ParseError::UnexpectedEof)
    }

    /// Advance the cursor, returning `Err(UnexpectedEof)` if we move past
    /// the end. Use when the grammar *requires* a token to follow.
    pub fn advance(&mut self) -> Result<(), ParseError> {
        self.cursor += 1;
        if self.is_eof() { Err(ParseError::UnexpectedEof) } else { Ok(()) }
    }

    /// Advance the cursor without failing at EOF.
    ///
    /// Use when reaching the end of the stream after consuming a token is
    /// legitimate
    ///
    /// (e.g. closing ')', last argument, numeric literal at the end of input).
    pub fn advance_unchecked(&mut self) {
        self.cursor += 1;
        // cursor may now == tokens.len(); that is fine — is_eof() will reflect
        // it
    }

    /// Skip one token for error recovery
    ///
    /// Returns `true` if there are more tokens to process.
    pub fn skip_for_recovery(&mut self) -> bool {
        self.cursor += 1;
        !self.is_eof()
    }

    /// Returns the precedence of the current token if it is a known binary
    /// operator, or `None` otherwise.
    #[must_use]
    pub fn tok_precedence(&self) -> Option<u8> {
        let TokenKind::Op(op) = self.current().ok()? else { return None };
        let p = self.prec.get(&op).copied()?;
        (p > 0).then_some(p)
    }

    /// Returns `true` if the cursor is at or past the end of the token slice.
    #[must_use]
    pub const fn is_eof(&self) -> bool { self.cursor >= self.tokens.len() }
}

#[cfg(test)]
mod tests {
    use core::assert_matches;

    use super::*;
    use crate::lexer::Lexer;

    fn make_parser<'a>(
        src: &'a str,
        tokens: &'a Vec<TokenKind<'a>>,
        prec: &'a HashMap<char, u8>,
    ) -> Parser<'a> {
        #[expect(clippy::no_effect_underscore_binding)]
        let _src = src; // held alive by caller
        Parser::new(tokens, prec)
    }

    fn default_prec() -> HashMap<char, u8> {
        [('=', 2u8), ('<', 10), ('+', 20), ('-', 20), ('*', 40), ('/', 40)].into_iter().collect()
    }

    #[test]
    fn parse_number() {
        let prec = default_prec();
        let src = "42";

        let tokens: Vec<_> = Lexer::new(src).tokens().collect();
        let mut p = make_parser(src, &tokens, &prec);

        let expr = p.parse_expr().unwrap();
        assert_matches!(expr, Expr::Number(_));
    }

    #[test]
    fn parse_binary_add() {
        let prec = default_prec();
        let src = "x+y";

        let tokens: Vec<_> = Lexer::new(src).tokens().collect();
        let mut p = make_parser(src, &tokens, &prec);

        let expr = p.parse_expr().unwrap();
        assert_matches!(expr, Expr::Binary { op: '+', .. });
    }

    #[test]
    fn parse_definition() {
        let prec = default_prec();
        let src = "def foo(x y) x+y";

        let tokens: Vec<_> = Lexer::new(src).tokens().collect();
        let mut p = make_parser(src, &tokens, &prec);

        let func = p.parse_definition().unwrap();
        assert_eq!(func.proto.name, "foo");
        assert_eq!(func.proto.args, vec!["x", "y"]);
        assert!(func.body.is_some());
    }

    #[test]
    fn parse_extern() {
        let prec = default_prec();
        let src = "extern sin(a)";

        let tokens: Vec<_> = Lexer::new(src).tokens().collect();
        let mut p = make_parser(src, &tokens, &prec);

        let func = p.parse_extern().unwrap();
        assert_eq!(func.proto.name, "sin");
        assert!(func.body.is_none());
    }

    #[test]
    fn parse_call_with_float_arg() {
        let prec = default_prec();
        let src = "foo(y, 4.0)";

        let tokens: Vec<_> = Lexer::new(src).tokens().collect();
        let mut p = make_parser(src, &tokens, &prec);

        let expr = p.parse_expr().unwrap();
        assert_matches!(expr, Expr::Call { .. });
    }

    #[test]
    fn anon_proto_name() {
        let prec = default_prec();
        let src = "1+2";

        let tokens: Vec<_> = Lexer::new(src).tokens().collect();
        let mut p = make_parser(src, &tokens, &prec);

        let func = p.parse_toplevel_expr().unwrap();
        assert_eq!(func.proto.name, "__anon_expr");
    }
}
