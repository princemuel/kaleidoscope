use std::io;

use kaleidoscope::token::TokenKind;
use kaleidoscope::{Error, Lexer, Parser};

const PRECENDENCE_OPS: [(char, u8); 6] =
    [('=', 2), ('<', 10), ('+', 20), ('-', 20), ('*', 40), ('/', 40)];

macro_rules! printf {
    ( $( $x:expr ),* ) => {
        use std::io::Write as _;
        print!( $($x, )* );

        std::io::stdout().flush().expect("Could not flush to stdout");
    };
}

fn main() -> Result<(), Error> {
    loop {
        println!();
        printf!("ready> ");

        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;

        let prec = PRECENDENCE_OPS.into();
        let tokens: Vec<_> = Lexer::new(&buffer).collect();
        let mut parser = Parser::new(&tokens, &prec);

        // Keep dispatching until the token stream is exhausted
        loop {
            match parser.current() {
                Err(_) => break,                          // EOF
                Ok(TokenKind::Eof) => break,
                Ok(TokenKind::Op(';')) => { parser.advance().ok(); }
                Ok(TokenKind::Def) => handle_definition(&mut parser),
                Ok(TokenKind::Extern) => handle_extern(&mut parser),
                _ => handle_toplevel_expr(&mut parser),
            }
        }

        // Check if the very first token was EOF (user hit Ctrl+D)
        if tokens.is_empty() || tokens[0] == TokenKind::Eof {
            break Ok(());
        }
    }
}

fn handle_definition(parser: &mut Parser<'_>) {
    match parser.parse_definition() {
        Ok(_) => eprintln!("Parsed a function definition."), // drop name
        Err(e) => eprintln!("Error in definition: {e}"),
    }
}

fn handle_extern(parser: &mut Parser<'_>) {
    match parser.parse_extern() {
        Ok(_) => eprintln!("Parsed an extern"), // drop name
        Err(e) => eprintln!("Error parsing extern: {e}"),
    }
}

fn handle_toplevel_expr(parser: &mut Parser<'_>) {
    match parser.parse_toplevel_expr() {
        Ok(_) => eprintln!("Parsed a top-level expr"),
        Err(e) => eprintln!("Error: {e}"),
    }
}
