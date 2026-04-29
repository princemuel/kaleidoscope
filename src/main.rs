#![expect(clippy::print_stdout)]
#![expect(unused)]
#![expect(unsafe_code)]
use std::collections::HashMap;
use std::io;

use clap::Parser as _;
use kaleidoscope::lexer::Lexer;
use kaleidoscope::parser::Parser;
use kaleidoscope::token::Token;

// ======================================================================================
// PROGRAM ==============================================================================
// ======================================================================================

// macro used to print & flush without printing a new line
macro_rules! print_flush {
    ( $( $x:expr ),* ) => {
        use std::io::Write as _;
        print!( $($x, )* );

        std::io::stdout().flush().expect("Could not flush to stdout");
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn putchard(x: f64) -> f64 {
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let c: char = (x.round() as u8).into();
    print_flush!("{c}");
    x
}

#[unsafe(no_mangle)]
pub extern "C" fn printd(x: f64) -> f64 {
    println!("{x}");
    x
}

// Adding the functions above to a global array,
// so Rust compiler won't remove them.
#[used]
static EXTERNAL_FNS: [extern "C" fn(f64) -> f64; 2] = [putchard, printd];

const PRECENDENCE_OPS: [(char, u8); 6] = [
    ('=', 2),
    ('<', 10),
    ('+', 20),
    ('-', 20),
    ('*', 40),
    ('/', 40),
];

#[derive(Debug, clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long = "dl")]
    display_lexer_output: bool,

    #[arg(long = "dp")]
    display_parser_output: bool,

    #[arg(long = "dc")]
    display_compiler_output: bool,

    #[arg(short = 'e')]
    eval: Option<String>,
}

/// Entry point of the program; acts as a REPL.
fn main() -> Result<(), Box<dyn core::error::Error>> {
    loop {
        println!();
        print_flush!("ready> ");

        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;

        #[expect(clippy::else_if_without_else)]
        if buffer.starts_with("exit") || buffer.starts_with("quit") {
            break Ok(());
        } else if buffer.chars().all(char::is_whitespace) {
            continue;
        }

        let mut prec = PRECENDENCE_OPS.into();
        let tokens = Lexer::new(&buffer).collect::<Result<Vec<_>, _>>()?;
        let mut parser = Parser::new(&tokens, &mut prec);

        match parser.current()? {
            Token::EOF => break Ok(()),
            Token::Op(';') => {
                parser.advance()?;
            }
            Token::Def => handle_definition(&mut parser),
            Token::Extern => handle_extern(&mut parser),
            _ => handle_toplevel_expr(&mut parser),
        }
    }
}

// ============================================================================
// REPL & HANDLERS
// ============================================================================

use std::io::Write as _;

fn handle_definition(parser: &mut Parser<'_>) {
    match parser.parse_definition() {
        Ok(func) => {
            eprintln!("Parsed a function definition: {}", func.proto.name);
        }
        Err(e) => eprintln!("Error in definition: {e:?}"),
    }
}

fn handle_extern(parser: &mut Parser<'_>) {
    match parser.parse_extern() {
        Ok(proto) => eprintln!("Parsed an extern: {}", proto.proto.name),
        Err(e) => eprintln!("Error parsing extern: {e:?}"),
    }
}

fn handle_toplevel_expr(parser: &mut Parser<'_>) {
    match parser.parse_toplevel_expr() {
        Ok(func) => {
            eprintln!("Parsed a top-level expr");
        }
        Err(e) => eprintln!("Error: {e:?}"),
    }
}
