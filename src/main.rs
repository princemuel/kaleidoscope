use kaleidoscope::lexer::Lexer;

fn main() {
    let stdin = std::io::stdin();
    let _lexer = Lexer::new(stdin.lock());

    eprint!("ready> ");
    run();
}

fn run() {}
