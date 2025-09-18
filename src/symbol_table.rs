use simplicityhl::error::Span;
use simplicityhl::parse::Program;
use simplicityhl::parse::{Expression, ExpressionInner, Function, Item, TypeAlias};

#[derive(Debug)]
enum SymbolType {
    Function,
    FunctionCall,
    Variable,
    FunctionParam,
    TypeAlias,
    ModuleVariable,
}

#[derive(Debug)]
struct SymbolInfo {
    pub ty: SymbolType,

    pub identifier: String,
    pub span: Span,
}

#[derive(Debug)]
struct SymbolTable {
    symbols: Vec<SymbolInfo>,
    idx_prev: Option<u32>,
}

impl SymbolTable {
    fn new() -> Self {
        Self {
            symbols: Vec::new(),
            idx_prev: None,
        }
    }

    fn with_parent(parent: u32) -> Self {
        Self {
            symbols: Vec::new(),
            idx_prev: Some(parent),
        }
    }

    fn insert(&mut self, symbol: SymbolInfo) {
        self.symbols.push(symbol);
    }

    fn lookup(&self, name: &str) -> Option<&SymbolInfo> {
        self.symbols.iter().rev().find(|s| s.identifier == name)
    }
}

#[derive(Debug)]
struct SemanticTree {
    symbol_tables: Vec<SymbolTable>,
}

impl SemanticTree {
    fn insert(&mut self, table: SymbolTable) {
        self.symbol_tables.push(table);
    }

    fn from_ast(program: Program) -> Self {
        let mut result = Self {
            symbol_tables: vec![],
        };

        // First table is global scope
        result.insert(SymbolTable::new());

        program.items().iter().for_each(|item| match item {
            Item::Function(func) => result.symbol_tables[0].insert(SymbolInfo {
                identifier: func.name().to_string(),
                span: func.span().to_owned(),
                ty: SymbolType::Function,
            }),
            Item::TypeAlias(alias) => result.symbol_tables[0].insert(SymbolInfo {
                identifier: alias.name().to_string(),
                span: alias.span().to_owned(),
                ty: SymbolType::TypeAlias,
            }),
            // Modules ignored in original SimplicityHL compiler
            Item::Module => {}
        });

        let functions = program
            .items()
            .iter()
            .filter(|item| matches!(item, Item::Function(_)))
            .collect::<Vec<&Item>>();

        result
    }
}
