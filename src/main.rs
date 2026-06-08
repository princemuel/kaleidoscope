use std::collections::HashMap;
use std::io;
use std::io::prelude::*;

use inkwell::context::Context;
use inkwell::targets::{InitializationConfig, Target};
use klyd::error::CodegenError;
use klyd::token::TokenKind;
use klyd::{Codegen, Error, Lexer, Parser};

/// Binary-operator precedence table.
const PRECEDENCE_OPS: [(char, u8); 6] =
    [('=', 2), ('<', 10), ('+', 20), ('-', 20), ('*', 40), ('/', 40)];

/// Print a prompt and flush stderr immediately, matching C++'s `fprintf(stderr,
/// "ready> ")`.
macro_rules! prompt {
    () => {{
        eprint!("ready> ");
        io::stderr().flush().expect("failed to flush stdout");
    }};
}

fn main() -> Result<(), Error> {
    let stdin = io::stdin();
    prompt!();

    // Make the module, which holds all the code.
    Target::initialize_native(&InitializationConfig::default()).map_err(CodegenError::Unknown)?;
    let context = Context::create();
    let mut codegen = Codegen::new("my cool jit", &context);
    let prec = HashMap::from(PRECEDENCE_OPS);

    for line in stdin.lock().lines() {
        let buffer = line?;

        // Empty line = Ctrl+D (EOF) on most terminals.
        if buffer.is_empty() {
            break;
        }

        // Lex the entire line into a Vec<TokenKind> upfront.
        // C++ lexes on-demand from stdin; we batch-lex one line at a time.
        // The semantics are identical for single-line input; multi-line
        // function bodies would require accumulation (future work).
        let tokens: Vec<_> = Lexer::new(&buffer).tokens().collect();

        // An empty token list (blank line or comment-only) — just re-prompt.
        if tokens.is_empty() {
            prompt!();
            continue;
        }

        let mut parser = Parser::new(&tokens, &prec);

        // Run the "interpreter loop".
        loop {
            match parser.current() {
                // Past the end of the token slice...be done with this line.
                Err(_) => break,

                // Semicolons are statement separators; skip silently.
                Ok(TokenKind::Op(';')) => {
                    parser.advance_unchecked();
                }

                Ok(TokenKind::Def) => handle_definition(&mut parser, &mut codegen),
                Ok(TokenKind::Extern) => handle_extern(&mut parser, &mut codegen),

                // Any other token: treat as a top-level expression.
                _ => handle_toplevel_expr(&mut parser, &mut codegen),
            }
        }

        // Print the prompt for the next line after finishing all dispatches.
        // This matches C++: the prompt is printed at the *top* of MainLoop's
        // while(true) body, i.e. before each new dispatch, which from the
        // user's perspective means "after the previous output".
        prompt!();
    }

    // Print out all of the generated code.
    codegen.module.print_to_stderr();

    Ok(())
}

// The following functions skip one token for error recovery On failure
// matching the C++ version.

/// Handle a `def` — parse a function definition and report success or failure.
fn handle_definition(parser: &mut Parser<'_>, codegen: &mut Codegen<'_>) {
    match parser.parse_definition() {
        Ok(func) => match codegen.function(&func) {
            Ok(fn_val) => {
                eprintln!("Read function definition:");
                fn_val.print_to_stderr();
            }
            Err(e) => eprintln!("Codegen error: {e}"),
        },
        Err(e) => {
            eprintln!("Error in definition: {e}");
            parser.skip_for_recovery();
        }
    }
}

/// Handle an `extern` declaration.
fn handle_extern(parser: &mut Parser<'_>, codegen: &mut Codegen<'_>) {
    match parser.parse_extern() {
        Ok(func) => match codegen.proto(&func.proto) {
            Ok(proto) => {
                eprintln!("Read extern: ");
                proto.print_to_stderr();
            }
            Err(e) => eprintln!("Codegen error: {e}"),
        },
        Err(e) => {
            eprintln!("Error parsing extern: {e}");
            parser.skip_for_recovery();
        }
    }
}
/// Handle a top-level expression.
fn handle_toplevel_expr(parser: &mut Parser<'_>, codegen: &mut Codegen<'_>) {
    match parser.parse_toplevel_expr() {
        Ok(func) => match codegen.function(&func) {
            Ok(fn_val) => {
                eprintln!("Read top-level expression:");
                fn_val.print_to_stderr();
                // erase so it doesn't conflict on the next top-level expr
                #[expect(unsafe_code)]
                unsafe {
                    fn_val.delete();
                };
            }
            Err(e) => eprintln!("Codegen error: {e}"),
        },
        Err(e) => {
            eprintln!("Error: {e}");
            parser.skip_for_recovery();
        }
    }
}
