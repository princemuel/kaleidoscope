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
    use core::{assert_matches, f64};

    use super::*;
    use crate::lexer::Lexer;

    fn default_prec() -> HashMap<char, u8> {
        [('=', 2u8), ('<', 10), ('>', 10), ('+', 20), ('-', 20), ('*', 40), ('/', 40)]
            .into_iter()
            .collect()
    }

    /// Lex `src` and return a parser over the resulting token slice.
    /// The token `Vec` is returned alongside so it outlives the parser.
    fn parser(src: &str) -> (Vec<TokenKind<'_>>, HashMap<char, u8>) {
        let tokens = Lexer::new(src).tokens().collect();
        let prec = default_prec();
        (tokens, prec)
    }

    /// Convenience: parse a full expression from `src`.
    macro_rules! parse_expr {
        ($src:expr) => {{
            let (tokens, prec) = parser($src);
            let mut p = Parser::new(&tokens, &prec);
            p.parse_expr()
        }};
    }

    /// Convenience: parse a definition from `src`.
    macro_rules! parse_def {
        ($src:expr) => {{
            let (tokens, prec) = parser($src);
            let mut p = Parser::new(&tokens, &prec);
            p.parse_definition()
        }};
    }

    /// Convenience: parse an extern from `src`.
    macro_rules! parse_extern {
        ($src:expr) => {{
            let (tokens, prec) = parser($src);
            let mut p = Parser::new(&tokens, &prec);
            p.parse_extern()
        }};
    }

    /// Convenience: parse a top-level expression from `src`.
    macro_rules! parse_toplevel {
        ($src:expr) => {{
            let (tokens, prec) = parser($src);
            let mut p = Parser::new(&tokens, &prec);
            p.parse_toplevel_expr()
        }};
    }

    #[test]
    fn current_returns_first_token() {
        let (tokens, prec) = parser("def");
        let p = Parser::new(&tokens, &prec);
        assert_eq!(p.current(), Ok(TokenKind::Def));
    }

    #[test]
    fn current_on_empty_slice_returns_eof() {
        let prec = default_prec();
        let p = Parser::new(&[], &prec);
        assert_matches!(p.current(), Err(ParseError::UnexpectedEof));
    }

    #[test]
    fn advance_moves_cursor() {
        let (tokens, prec) = parser("def extern");
        let mut p = Parser::new(&tokens, &prec);
        p.advance().unwrap();
        assert_eq!(p.current(), Ok(TokenKind::Extern));
    }

    #[test]
    fn advance_past_end_returns_eof_error() {
        let (tokens, prec) = parser("x");
        let mut p = Parser::new(&tokens, &prec);
        let err = p.advance().unwrap_err();
        assert_matches!(err, ParseError::UnexpectedEof);
    }

    #[test]
    fn advance_unchecked_does_not_fail_at_eof() {
        let (tokens, prec) = parser("x");
        let mut p = Parser::new(&tokens, &prec);
        p.advance_unchecked(); // moves past 'x'
        assert!(p.is_eof());
        p.advance_unchecked(); // must not panic
        assert!(p.is_eof());
    }

    #[test]
    fn is_eof_false_when_tokens_remain() {
        let (tokens, prec) = parser("x");
        let p = Parser::new(&tokens, &prec);
        assert!(!p.is_eof());
    }

    #[test]
    fn is_eof_true_after_all_tokens_consumed() {
        let (tokens, prec) = parser("x");
        let mut p = Parser::new(&tokens, &prec);
        p.advance_unchecked();
        assert!(p.is_eof());
    }

    #[test]
    fn skip_for_recovery_advances_and_returns_true_when_more_tokens() {
        let (tokens, prec) = parser("x y");
        let mut p = Parser::new(&tokens, &prec);
        let more = p.skip_for_recovery();
        assert!(more);
        assert_eq!(p.current(), Ok(TokenKind::Ident("y")));
    }

    #[test]
    fn skip_for_recovery_returns_false_at_eof() {
        let (tokens, prec) = parser("x");
        let mut p = Parser::new(&tokens, &prec);
        let more = p.skip_for_recovery();
        assert!(!more);
    }

    #[test]
    fn tok_precedence_known_op() {
        let (tokens, prec) = parser("+");
        let p = Parser::new(&tokens, &prec);
        assert_eq!(p.tok_precedence(), Some(20));
    }

    #[test]
    fn tok_precedence_unknown_op() {
        let (tokens, prec) = parser("%");
        let p = Parser::new(&tokens, &prec);
        assert_eq!(p.tok_precedence(), None);
    }

    #[test]
    fn tok_precedence_non_op_token() {
        let (tokens, prec) = parser("def");
        let p = Parser::new(&tokens, &prec);
        assert_eq!(p.tok_precedence(), None);
    }

    #[test]
    fn num_expr_integer() {
        let expr = parse_expr!("42").unwrap();
        assert_eq!(expr, Expr::Number(42.0));
    }

    #[test]
    #[expect(clippy::approx_constant)]
    fn num_expr_float() {
        let expr = parse_expr!("3.14").unwrap();
        assert_matches!(expr, Expr::Number(v) if (v - 3.14).abs() < f64::EPSILON);
    }

    #[test]
    fn num_expr_zero() {
        let expr = parse_expr!("0").unwrap();
        assert_eq!(expr, Expr::Number(0.0));
    }

    #[test]
    fn ident_expr_plain_variable() {
        let expr = parse_expr!("x").unwrap();
        assert_eq!(expr, Expr::Variable("x".to_owned()));
    }

    #[test]
    fn ident_expr_call_no_args() {
        let expr = parse_expr!("foo()").unwrap();
        assert_eq!(expr, Expr::Call { name: "foo".to_owned(), args: vec![] });
    }

    #[test]
    fn ident_expr_call_one_arg() {
        let expr = parse_expr!("sin(x)").unwrap();
        assert_eq!(expr, Expr::Call {
            name: "sin".to_owned(),
            args: vec![Expr::Variable("x".to_owned())]
        });
    }

    #[test]
    fn ident_expr_call_multiple_args() {
        let expr = parse_expr!("foo(a, b, c)").unwrap();
        assert_eq!(expr, Expr::Call {
            name: "foo".to_owned(),
            args: vec![
                Expr::Variable("a".to_owned()),
                Expr::Variable("b".to_owned()),
                Expr::Variable("c".to_owned()),
            ]
        });
    }

    #[test]
    fn ident_expr_call_with_expr_arg() {
        // foo(1+2) — arg is itself a binary expression
        let expr = parse_expr!("foo(1+2)").unwrap();
        assert_matches!(
            expr,
            Expr::Call { ref name, ref args }
            if name == "foo" && args.len() == 1
        );
    }

    #[test]
    fn ident_expr_call_missing_rparen_returns_error() {
        let err = parse_expr!("foo(a, b").unwrap_err();
        assert_matches!(err, ParseError::UnexpectedEof);
    }

    #[test]
    fn ident_expr_call_bad_separator_returns_error() {
        let err = parse_expr!("foo(a b)").unwrap_err();
        assert_matches!(err, ParseError::ExpectedCommaOrRParen(_));
    }

    // ── parse_paren_expr ──────────────────────────────────────────────────────

    #[test]
    fn paren_expr_simple() {
        let expr = parse_expr!("(42)").unwrap();
        assert_eq!(expr, Expr::Number(42.0));
    }

    #[test]
    fn paren_expr_nested() {
        let expr = parse_expr!("((x))").unwrap();
        assert_eq!(expr, Expr::Variable("x".to_owned()));
    }

    #[test]
    fn paren_expr_with_binop() {
        let expr = parse_expr!("(1+2)").unwrap();
        assert_matches!(expr, Expr::Binary { op: '+', .. });
    }

    #[test]
    fn paren_expr_missing_rparen_returns_error() {
        let err = parse_expr!("(x").unwrap_err();
        assert_matches!(err, ParseError::UnexpectedEof | ParseError::ExpectedRParen(_));
    }

    // ── parse_bin_expr (Pratt / precedence climbing) ──────────────────────────

    #[test]
    fn binop_add() {
        let expr = parse_expr!("x+y").unwrap();
        assert_eq!(expr, Expr::Binary {
            op: '+',
            lhs: Box::new(Expr::Variable("x".to_owned())),
            rhs: Box::new(Expr::Variable("y".to_owned())),
        });
    }

    #[test]
    fn binop_left_associative() {
        // x+y+z should parse as (x+y)+z
        let expr = parse_expr!("x+y+z").unwrap();
        assert_matches!(
            expr,
            Expr::Binary {
                op: '+',
                lhs,
                ..
            }
            if matches!(*lhs, Expr::Binary { op: '+', .. })
        );
    }

    #[test]
    fn binop_precedence_mul_over_add() {
        // x+y*z should parse as x+(y*z)
        let expr = parse_expr!("x+y*z").unwrap();
        assert_matches!(
            expr,
            Expr::Binary {
                op: '+',
                rhs,
                ..
            }
            if matches!(*rhs, Expr::Binary { op: '*', .. })
        );
    }

    #[test]
    fn binop_precedence_parens_override() {
        // (x+y)*z — parens make + bind tighter
        let expr = parse_expr!("(x+y)*z").unwrap();
        assert_matches!(
            expr,
            Expr::Binary {
                op: '*',
                lhs,
                ..
            }
            if matches!(*lhs, Expr::Binary { op: '+', .. })
        );
    }

    #[test]
    fn binop_all_arithmetic_ops_parse() {
        for op in ['+', '-', '*', '/'] {
            let src = format!("x{op}y");
            let expr = parse_expr!(&src).unwrap();
            assert!(
                matches!(expr, Expr::Binary { op: o, .. } if o == op),
                "operator '{op}' failed to parse"
            );
        }
    }

    #[test]
    fn binop_comparison_lt() {
        let expr = parse_expr!("x<y").unwrap();
        assert_matches!(expr, Expr::Binary { op: '<', .. });
    }

    #[test]
    fn binop_unknown_op_not_consumed() {
        // '%' is not in the precedence table — the parser should stop and
        // return just `x`, leaving '%' unconsumed.
        let (tokens, prec) = parser("x%y");
        let mut p = Parser::new(&tokens, &prec);
        let expr = p.parse_expr().unwrap();
        assert_eq!(expr, Expr::Variable("x".to_owned()));
        // cursor must be sitting on '%'
        assert_eq!(p.current(), Ok(TokenKind::Op('%')));
    }

    #[test]
    fn binop_complex_expression() {
        // a*a + 2*a*b + b*b — the tutorial's quadratic test case
        let expr = parse_expr!("a*a + 2*a*b + b*b").unwrap();
        // Top level must be '+'
        assert_matches!(expr, Expr::Binary { op: '+', .. });
    }

    #[test]
    fn primary_dispatches_to_number() {
        let expr = parse_expr!("1").unwrap();
        assert_matches!(expr, Expr::Number(_));
    }

    #[test]
    fn primary_dispatches_to_ident() {
        let expr = parse_expr!("x").unwrap();
        assert_matches!(expr, Expr::Variable(_));
    }

    #[test]
    fn primary_dispatches_to_paren() {
        let expr = parse_expr!("(1)").unwrap();
        assert_matches!(expr, Expr::Number(_));
    }

    #[test]
    fn primary_unknown_token_returns_error() {
        // ';' is not a valid primary expression start
        let err = parse_expr!(";").unwrap_err();
        assert_matches!(err, ParseError::ExpectedExpr(_));
    }

    #[test]
    fn prototype_no_args() {
        let func = parse_extern!("extern foo()").unwrap();
        assert_eq!(func.proto.name, "foo");
        assert!(func.proto.args.is_empty());
    }

    #[test]
    fn prototype_one_arg() {
        let func = parse_extern!("extern sin(x)").unwrap();
        assert_eq!(func.proto.args, vec!["x"]);
    }

    #[test]
    fn prototype_multiple_args() {
        let func = parse_extern!("extern foo(a b c)").unwrap();
        assert_eq!(func.proto.args, vec!["a", "b", "c"]);
    }

    #[test]
    fn prototype_missing_lparen_returns_error() {
        let err = parse_extern!("extern foo x)").unwrap_err();
        assert_matches!(err, ParseError::ExpectedLParen(_));
    }

    #[test]
    fn prototype_missing_rparen_returns_error() {
        let err = parse_extern!("extern foo(x").unwrap_err();
        assert_matches!(err, ParseError::UnexpectedEof | ParseError::ExpectedRParen(_));
    }

    #[test]
    fn prototype_missing_name_returns_error() {
        let err = parse_extern!("extern (x)").unwrap_err();
        assert_matches!(err, ParseError::ExpectedPrototypeName(_));
    }

    #[test]
    fn definition_simple() {
        let func = parse_def!("def foo(x) x").unwrap();
        assert_eq!(func.proto.name, "foo");
        assert_eq!(func.proto.args, vec!["x"]);
        assert_eq!(func.body, Some(Expr::Variable("x".to_owned())));
    }

    #[test]
    fn definition_two_args_with_body() {
        let func = parse_def!("def add(a b) a+b").unwrap();
        assert_eq!(func.proto.args, vec!["a", "b"]);
        assert_matches!(func.body, Some(Expr::Binary { op: '+', .. }));
    }

    #[test]
    fn definition_body_is_some() {
        let func = parse_def!("def id(x) x").unwrap();
        assert!(func.body.is_some());
    }

    #[test]
    fn definition_missing_body_returns_error() {
        let err = parse_def!("def foo(x)").unwrap_err();
        assert_matches!(err, ParseError::UnexpectedEof);
    }

    #[test]
    fn definition_missing_prototype_returns_error() {
        let err = parse_def!("def (x) x").unwrap_err();
        assert_matches!(err, ParseError::ExpectedPrototypeName(_));
    }

    #[test]
    fn extern_body_is_none() {
        let func = parse_extern!("extern cos(x)").unwrap();
        assert!(func.body.is_none());
    }

    #[test]
    fn extern_proto_name_correct() {
        let func = parse_extern!("extern atan2(y x)").unwrap();
        assert_eq!(func.proto.name, "atan2");
        assert_eq!(func.proto.args, vec!["y", "x"]);
    }

    #[test]
    fn toplevel_wraps_in_anon_proto() {
        let func = parse_toplevel!("4+5").unwrap();
        assert_eq!(func.proto.name, "__anon_expr");
        assert!(func.proto.args.is_empty());
    }

    #[test]
    fn toplevel_body_is_the_expression() {
        let func = parse_toplevel!("42").unwrap();
        assert_eq!(func.body, Some(Expr::Number(42.0)));
    }

    #[test]
    fn toplevel_complex_expression() {
        let func = parse_toplevel!("foo(a, 4.0) + bar(31337)").unwrap();
        assert_matches!(func.body, Some(Expr::Binary { op: '+', .. }));
    }

    #[test]
    fn multi_dispatch_def_then_toplevel() {
        // Simulates how main.rs drives the parser loop over one line.
        let (tokens, prec) = parser("def foo(x y) x+y y");
        let mut p = Parser::new(&tokens, &prec);

        let def = p.parse_definition().unwrap();
        assert_eq!(def.proto.name, "foo");

        let top = p.parse_toplevel_expr().unwrap();
        assert_eq!(top.body, Some(Expr::Variable("y".to_owned())));
    }

    #[test]
    fn multi_dispatch_extern_then_call() {
        let (tokens, prec) = parser("extern sin(x) sin(1)");
        let mut p = Parser::new(&tokens, &prec);

        let ext = p.parse_extern().unwrap();
        assert_eq!(ext.proto.name, "sin");

        let top = p.parse_toplevel_expr().unwrap();
        assert_matches!(top.body, Some(Expr::Call { .. }));
    }

    #[test]
    fn skip_for_recovery_then_parse_succeeds() {
        // Simulate a bad token followed by a valid expression.
        let (tokens, prec) = parser(") x");
        let mut p = Parser::new(&tokens, &prec);

        // First parse fails on ')'.
        let err = p.parse_expr();
        assert!(err.is_err());

        // Recovery: skip the bad token.
        p.skip_for_recovery();

        // Next parse should succeed.
        let expr = p.parse_expr().unwrap();
        assert_eq!(expr, Expr::Variable("x".to_owned()));
    }

    #[test]
    fn cloned_parser_is_independent() {
        let (tokens, prec) = parser("x y");
        let original = Parser::new(&tokens, &prec);
        let mut clone = original.clone();
        let mut original = original;

        // Advance original — clone must be unaffected.
        original.advance_unchecked();
        assert_eq!(original.current(), Ok(TokenKind::Ident("y")));
        assert_eq!(clone.current(), Ok(TokenKind::Ident("x")));

        // And vice versa.
        clone.advance_unchecked();
        assert_eq!(clone.current(), Ok(TokenKind::Ident("y")));
    }
}
