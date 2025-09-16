use simplicityhl::error::Span;
use simplicityhl::parse::{Function, TypeAlias};

enum SymbolType {
    Function,
    Variable,
    FunctionParam,
    TypeAlias,
    ModuleVariable,
}

struct SymbolInfo {
    ty: SymbolType,

    identifier: String,
    span: Span,
}

struct SymbolTable {
    symbols: Vec<SymbolInfo>,
    idx_prev: Option<u32>,
}
