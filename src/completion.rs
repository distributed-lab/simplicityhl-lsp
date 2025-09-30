use simplicityhl::jet;
use simplicityhl::parse::Function;
use simplicityhl::simplicity::jet::Elements;

use crate::jet::documentation;

use tower_lsp_server::lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat,
};

#[derive(Debug)]
pub struct CompletionProvider {
    jets_completion: Vec<CompletionItem>,
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
        documentation: Some(Documentation::String(documentation(jet).to_string())),
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

// TODO: too many nested blocks (refactor)
impl CompletionProvider {
    pub fn new() -> Self {
        let jets_completion = Elements::ALL
            .iter()
            .map(|jet| jet_to_completion_item(*jet))
            .collect();

        Self { jets_completion }
    }

    pub fn get_jets(&self) -> Vec<CompletionItem> {
        self.jets_completion.clone()
    }

    pub fn get_function_completions(functions: &[Function]) -> Vec<CompletionItem> {
        functions.iter().map(function_to_completion_item).collect()
    }
}
