/// The lexer returns tokens [0-255] if it is an unknown character, otherwise
/// one of these for known things.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Binary,
    Comma,
    Comment,
    Def,
    EOF,
    Extern,
    Ident(String),
    LParen,
    Number(f64),
    Op(char),
    RParen,
    // ! remeber to update `Lexer:lex_ident`
}
