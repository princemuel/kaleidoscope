pub type Result<T> = core::result::Result<T, String>;

pub enum Error {
    Lexer(String),
    Parse(String),
    Codegen(String),
    Jit(String),
    Io(std::io::Error),
}
