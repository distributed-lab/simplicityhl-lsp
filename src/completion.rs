use simplicityhl::jet;
use simplicityhl::parse::Function;
use simplicityhl::simplicity::jet::Elements;

use crate::jet::documentation;

use tower_lsp_server::lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
};

#[derive(Debug)]
pub struct CompletionProvider {
    jets_completion: Vec<CompletionItem>,
    builtin_completion: Vec<CompletionItem>,
}

#[derive(Debug, Clone)]
pub struct FunctionCompletionTemplate {
    /// Display name shown in completion list
    display_name: &'static str,
    /// Base name for snippet
    snippet_base: &'static str,
    /// Generic type parameters to include and use with snippet base
    generics: &'static [&'static str],
    /// Function arguments
    args: &'static [&'static str],
    /// Return type
    return_type: &'static str,
    /// Documentation
    description: &'static str,
}

impl FunctionCompletionTemplate {
    const fn new(
        display_name: &'static str,
        snippet_base: &'static str,
        generics: &'static [&'static str],
        args: &'static [&'static str],
        return_type: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            display_name,
            snippet_base,
            generics,
            args,
            return_type,
            description,
        }
    }

    const fn simple(
        display_name: &'static str,
        args: &'static [&'static str],
        return_type: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            display_name,
            snippet_base: display_name,
            generics: &[],
            args,
            return_type,
            description,
        }
    }

    fn generate_snippet_name(&self) -> String {
        format!(
            "{}::<{}>",
            self.snippet_base,
            self.generics
                .iter()
                .enumerate()
                .map(|(index, item)| { format!("${{{}:{}}}", index + 1, item) })
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
    fn get_insert_text(&self) -> String {
        format!(
            "{}({})",
            if self.generics.is_empty() {
                self.snippet_base.to_string()
            } else {
                self.generate_snippet_name()
            },
            self.args
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    format!("${{{}:{}}}", index + 1 + self.generics.len(), item)
                })
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
    fn get_signature(&self) -> String {
        format!(
            "fn({}) -> {}",
            self.args
                .iter()
                .copied()
                .map(str::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            if self.return_type.is_empty() {
                "()"
            } else {
                self.return_type
            }
        )
    }
}

static BUILTIN_FUNCTIONS: [FunctionCompletionTemplate; 9] = [
    FunctionCompletionTemplate::simple(
        "assert!",
        &["bool"],
        "",
        "Fails program if argument is 'false'",
    ),
    FunctionCompletionTemplate::simple("dbg!", &["type"], "type", "Print value and return it"),
    FunctionCompletionTemplate::simple("panic!", &[], "", "Fails program"),
    FunctionCompletionTemplate::new(
        "unwrap_left::<T>",
        "unwrap_left",
        &["T"],
        &["Either<T, U>"],
        "T",
        "Unwrap left side of Either",
    ),
    FunctionCompletionTemplate::new(
        "unwrap_right::<U>",
        "unwrap_right",
        &["U"],
        &["Either<T, U>"],
        "U",
        "Unwrap right side of Either",
    ),
    FunctionCompletionTemplate::new(
        "is_none::<T>",
        "is_none",
        &["T"],
        &["Option<T>"],
        "bool",
        "Check if Option is None",
    ),
    FunctionCompletionTemplate::new(
        "fold::<F, B>",
        "fold",
        &["F", "B"],
        &["iter", "init"],
        "B",
        "Fold operation over an iterator",
    ),
    FunctionCompletionTemplate::new(
        "array_fold::<F, N>",
        "array_fold",
        &["F", "N"],
        &["array", "init"],
        "B",
        "Fold operation over an array of size N",
    ),
    FunctionCompletionTemplate::new(
        "for_while::<F>",
        "for_while",
        &["F"],
        &["condition", "body"],
        "()",
        "While loop with a function",
    ),
];

impl CompletionProvider {
    pub fn new() -> Self {
        let jets_completion = Elements::ALL
            .iter()
            .copied()
            .map(jet_to_completion_item)
            .collect();
        let builtin_completion = BUILTIN_FUNCTIONS
            .iter()
            .map(builtint_to_completion_item)
            .collect();

        Self {
            jets_completion,
            builtin_completion,
        }
    }

    pub fn jets(&self) -> &[CompletionItem] {
        &self.jets_completion
    }

    pub fn builtins(&self) -> &[CompletionItem] {
        &self.builtin_completion
    }

    pub fn get_function_completions(functions: &[Function]) -> Vec<CompletionItem> {
        functions.iter().map(function_to_completion_item).collect()
    }
}

fn jet_to_completion_item(jet: Elements) -> CompletionItem {
    let name = jet.to_string();
    CompletionItem {
        label: name.clone(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some(format!(
            "fn({}) -> {}",
            jet::source_type(jet)
                .iter()
                .map(|item| { format!("{item}") })
                .collect::<Vec<_>>()
                .join(", "),
            jet::target_type(jet)
        )),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: documentation(jet).to_string(),
        })),
        insert_text: Some(format!(
            "{}({})",
            name,
            jet::source_type(jet)
                .iter()
                .enumerate()
                .map(|(index, item)| { format!("${{{}:{}}}", index + 1, item) })
                .collect::<Vec<_>>()
                .join(", ")
        )),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
    }
}

fn function_to_completion_item(func: &Function) -> CompletionItem {
    let name = func.name().to_string();
    CompletionItem {
        label: name.clone(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some(format!(
            "fn({}) -> {}",
            func.params()
                .iter()
                .map(|item| { format!("{}", item.ty()) })
                .collect::<Vec<_>>()
                .join(", "),
            match func.ret() {
                Some(ret) => format!("{ret}"),
                None => "()".to_string(),
            }
        )),
        insert_text: Some(format!(
            "{}({})",
            name,
            func.params()
                .iter()
                .enumerate()
                .map(|(index, item)| { format!("${{{}:{}}}", index + 1, item) })
                .collect::<Vec<_>>()
                .join(", ")
        )),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
    }
}

fn builtint_to_completion_item(func: &FunctionCompletionTemplate) -> CompletionItem {
    CompletionItem {
        label: func.display_name.to_string(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some(func.get_signature()),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: func.description.to_string(),
        })),
        insert_text: Some(func.get_insert_text()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
    }
}
