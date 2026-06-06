#[derive(Clone, Copy, Debug, PartialEq)]
enum TokenKind<'a> {
    Eof,
    Def,
    Extern,
    Ident(&'a str),
    Number(f64),
}
