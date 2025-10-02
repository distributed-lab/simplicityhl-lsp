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

struct FunctionCompletionTemplate {
    name: &'static str,
    args: &'static [&'static str],
    return_type: &'static str,
    description: &'static str,
}

// TODO: refactor and move this to another file
impl FunctionCompletionTemplate {
    const fn new(
        name: &'static str,
        args: &'static [&'static str],
        return_type: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            name,
            args,
            return_type,
            description,
        }
    }
}

static BUILTIN_FUNCTIONS: [FunctionCompletionTemplate; 4] = [
    FunctionCompletionTemplate::new(
        "assert!",
        &["bool"],
        "",
        "Fails program if argument is 'false'",
    ),
    FunctionCompletionTemplate::new("dbg!", &["type"], "type", "Print value and return it"),
    FunctionCompletionTemplate::new("unwrap", &["Option<ty>"], "ty", "Unwrap Option<type>"),
    FunctionCompletionTemplate::new("panic!", &[], "", "Fails program"),
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
    let name = func.name;
    CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some(format!(
            "fn({}) -> {}",
            func.args
                .iter()
                .copied()
                .map(str::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            func.return_type
        )),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: func.description.to_string(),
        })),
        insert_text: Some(format!(
            "{}({})",
            name,
            func.args
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
