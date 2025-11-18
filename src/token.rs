/// The lexer returns tokens [0-255] if it is an unknown character, otherwise one
/// of these for known things.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Eof,
    // commands
    Def,
    Extern,
    // primary
    Identifier(String),
    Number(f64),
    Char(char),
}
