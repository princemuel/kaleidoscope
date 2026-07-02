#![expect(unsafe_code)]
use std::collections::HashMap;
use std::io;
use std::io::prelude::*;

use inkwell::OptimizationLevel;
use inkwell::context::Context;
use inkwell::targets::{InitializationConfig, Target};
use kcc::codegen::{ANON_FN, SymbolTable};
use kcc::token::Kind;
use kcc::{CodeGen, Error, Lexer, Parser};

/// Binary-operator precedence table.
const PRECEDENCE_OPS: [(char, u8); 6] =
    [('=', 2), ('<', 10), ('+', 20), ('-', 20), ('*', 40), ('/', 40)];

/// Print a prompt and flush stderr immediately, matching C++'s `fprintf(stderr,
/// "ready> ")`.
macro_rules! prompt {
     ( $( $x:expr ),* ) => {
        eprint!( $($x, )* );
        std::io::stderr().flush().expect("failed to flush stdout");
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn putchard(x: f64) -> f64 {
    // let x = unsafe { x.abs().to_int_unchecked::<u8>() };
    #[expect(clippy::as_conversions, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let c = char::from(x.round().abs() as u8);
    prompt!("{c}");
    x
}

#[unsafe(no_mangle)]
pub extern "C" fn printd(x: f64) -> f64 {
    println!("{x}");
    x
}

// Add the fns above to a global array, so Rust compiler won't remove them.
pub static EXTERN_FNS: [extern "C" fn(f64) -> f64; 2] = [putchard, printd];

fn main() -> Result<(), Error> {
    let stdin = io::stdin();
    prompt!(">>> ");

    Target::initialize_native(&InitializationConfig::default()).map_err(Error::Unknown)?;

    let context = Context::create();

    let module = context.create_module("kcc");
    // module.run_passes(passes, machine, options)

    let engine = module.create_jit_execution_engine(OptimizationLevel::Aggressive)?;
    let target_data = engine.get_target_data();
    let data_layout = target_data.get_data_layout();

    module.set_data_layout(&data_layout);

    let mut codegen = CodeGen {
        context: &context,
        module,
        builder: context.create_builder(),
        engine,
        symbols: SymbolTable::default(),
    };

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
            prompt!(">>> ");
            continue;
        }

        let mut parser = Parser::new(&tokens, &prec);

        // Run the "interpreter loop".
        loop {
            match parser.current() {
                // Past the end of the token slice...be done with this line.
                Err(_) => break,

                // Semicolons are statement separators; skip silently.
                Ok(Kind::Op(';')) => {
                    parser.advance_unchecked();
                }

                Ok(Kind::Def) => handle_definition(&mut parser, &mut codegen),
                Ok(Kind::Extern) => handle_extern(&mut parser, &mut codegen),

                // Any other token: treat as a top-level expression.
                _ => handle_toplevel_expr(&mut parser, &mut codegen),
            }
        }

        // Print the prompt for the next line after finishing all dispatches.
        // This matches C++: the prompt is printed at the *top* of MainLoop's
        // while(true) body, i.e. before each new dispatch, which from the
        // user's perspective means "after the previous output".
        prompt!(">>> ");
    }

    // Print out all of the generated code.
    codegen.module.print_to_stderr();

    Ok(())
}

// The following functions skip one token for error recovery On failure
// matching the C++ version.

/// Handle a `def` — parse a function definition and report success or failure.
fn handle_definition(parser: &mut Parser<'_>, codegen: &mut CodeGen<'_>) {
    match parser.parse_definition() {
        Ok(func) => match codegen.func(&func) {
            Ok(fn_val) => {
                eprintln!("Read function definition:");
                fn_val.print_to_stderr();
            }
            Err(e) => eprintln!("Error: {e}"),
        },
        Err(e) => {
            eprintln!("Error in definition: {e}");
            parser.skip_for_recovery();
        }
    }
}

/// Handle an `extern` declaration.
fn handle_extern(parser: &mut Parser<'_>, codegen: &mut CodeGen<'_>) {
    match parser.parse_extern() {
        Ok(func) => match codegen.proto(&func.proto) {
            Ok(proto) => {
                eprintln!("Read extern: ");
                proto.print_to_stderr();
            }
            Err(e) => eprintln!("Error: {e}"),
        },
        Err(e) => {
            eprintln!("Error parsing extern: {e}");
            parser.skip_for_recovery();
        }
    }
}

/// Handle a top-level expression: codegen, optimize, JIT-execute, print result.
fn handle_toplevel_expr(parser: &mut Parser<'_>, codegen: &mut CodeGen<'_>) {
    match parser.parse_toplevel_expr() {
        Ok(func) => match codegen.func(&func) {
            Ok(fn_val) => {
                eprintln!("Read top-level expression:");
                fn_val.print_to_stderr();

                match codegen.run_anon(fn_val) {
                    Ok(result) => eprintln!("Evaluated to {result}"),
                    Err(e) => eprintln!("Error executing: {e}"),
                }
            }
            Err(e) => eprintln!("Error: {e}"),
        },
        Err(e) => {
            eprintln!("Error: {e}");
            parser.skip_for_recovery();
        }
    }
}
