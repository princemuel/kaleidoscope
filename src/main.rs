#![allow(unused)]
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

        std::io::stdout().flush().expect("Could not flush to standard output.");
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn putchard(x: f64) -> f64 {
    print_flush!("{}", x as u8 as char);
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
fn main() -> io::Result<()> {
    // let Args {
    //     display_lexer_output,
    //     display_parser_output,
    //     display_compiler_output,
    //     eval,
    //     ..
    // } = Args::parse();

    // let mut compute = |input: String| {
    //     let precendence = [('=', 2), ('<', 10), ('+', 20), ('-', 20), ('*', 40),
    // ('/', 40)];     let mut prec = HashMap::from_iter(precendence);

    //     // Parse and (optionally) display input
    //     if display_lexer_output {
    //         println!(
    //             "-> Attempting to parse lexed input: \n{:?}\n",
    //             Lexer::new(&input).collect::<Vec<Token>>()
    //         );
    //     }

    //     let (function, is_anonymous) = match Parser::new(input, &mut
    // prec).parse() {         Ok(func) => {
    //             let is_anon = func.is_anon;

    //             if display_parser_output {
    //                 if is_anon {
    //                     println!("-> Expression parsed: \n{:?}\n", func.body);
    //                 } else {
    //                     println!("-> Function parsed: \n{func:?}\n");
    //                 }
    //             }

    //             (function, is_anon)
    //         },
    //         Err(err) => {
    //             println!("!> Error parsing expression: {err}");
    //             return;
    //         },
    //     };

    //     if display_compiler_output {
    //         println!("-> Expression compiled to IR:");
    //         function.print_to_stderr();
    //     }
    // };
    // let stdin = std::io::stdin();
    // let lexer = Lexer::new(stdin.lock().into());
    // let mut parser = Parser::new(lexer)?;

    // // Prime the first token.
    // eprint!("ready> ");
    // parser.get_next_token()?;

    // // Run the main "interpreter loop" now.
    // run_main_loop();
    // if let Some(input) = eval {
    //     compute(format!("{input}\n"));
    // } else {
    //     loop {
    //         println!();
    //         print_flush!("?> ");

    //         let mut buffer = String::new();
    //         io::stdin()
    //             .read_line(&mut buffer)
    //             .expect("Could not read from standard input.");

    //         if buffer.starts_with("exit") || buffer.starts_with("quit") {
    //             break;
    //         } else if buffer.chars().all(char::is_whitespace) {
    //             continue;
    //         }

    //         compute(buffer);
    //     }
    // }

    loop {
        println!();
        print_flush!("?> ");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Could not read from standard input.");

        if input.starts_with("exit") || input.starts_with("quit") {
            break Ok(());
        } else if input.chars().all(char::is_whitespace) {
            continue;
        }

        let precendence = [('=', 2), ('<', 10), ('+', 20), ('-', 20), ('*', 40), ('/', 40)];
        let mut prec = HashMap::from_iter(precendence);
        let mut parser = Parser::new(&input, &mut prec);

        match parser.current()? {
            Token::EOF => break Ok(()),
            Token::Op(';') => {
                parser.advance()?;
            },
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

fn handle_definition(parser: &mut Parser) {
    match parser.parse_definition() {
        Ok(func) => {
            eprintln!("Parsed a function definition: {}", func.proto.name);
        },
        Err(e) => eprintln!("Error in definition: {:?}", e),
    }
}

fn handle_extern(parser: &mut Parser) {
    match parser.parse_extern() {
        Ok(proto) => eprintln!("Parsed an extern: {}", proto.proto.name),
        Err(e) => eprintln!("Error parsing extern: {:?}", e),
    }
}

fn handle_toplevel_expr(parser: &mut Parser) {
    match parser.parse_toplevel_expr() {
        Ok(func) => {
            eprintln!("Parsed a top-level expr");
        },
        Err(e) => eprintln!("Error: {:?}", e),
    }
}
