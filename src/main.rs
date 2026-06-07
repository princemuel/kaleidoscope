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
        match parser.current()? {
            TokenKind::Eof => break Ok(()),
            TokenKind::Op(';') => {
                parser.advance()?;
            }
            TokenKind::Def => handle_definition(&mut parser),
            TokenKind::Extern => handle_extern(&mut parser),
            _ => handle_toplevel_expr(&mut parser),
        }
    }
}

fn handle_definition(parser: &mut Parser<'_>) {
    match parser.parse_definition() {
        Ok(v) => {
            eprintln!("Parsed a function definition: {}", v.proto.name);
        }
        Err(e) => eprintln!("Error in definition: {e}"),
    }
}

fn handle_extern(parser: &mut Parser<'_>) {
    match parser.parse_extern() {
        Ok(v) => eprintln!("Parsed an extern: {}", v.proto.name),
        Err(e) => eprintln!("Error parsing extern: {e}"),
    }
}

fn handle_toplevel_expr(parser: &mut Parser<'_>) {
    match parser.parse_toplevel_expr() {
        Ok(_) => eprintln!("Parsed a top-level expr"),
        Err(e) => eprintln!("Error: {e}"),
    }
}
