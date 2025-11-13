use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum Token {
    #[token(":")]
    Colon,

    #[token("::")]
    DoubleColon,

    #[token("<")]
    OpenAngle,

    #[token(">")]
    CloseAngle,

    #[token("=")]
    EqualSign,

    #[regex(r"\(\s*[a-zA-Z_][a-zA-Z0-9_]*\s*,\s*[a-zA-Z_][a-zA-Z0-9_]*\s*\)", |lex| lex.slice().to_string())]
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),

    #[regex(r"jet::[a-zA-Z0-9_]?", priority = 2)]
    #[token("jet::", priority = 1)]
    Jet,
}
