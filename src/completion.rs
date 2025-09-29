use simplicityhl::jet;
use simplicityhl::parse::Function;
use simplicityhl::simplicity::jet::Elements;

use tower_lsp_server::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat};

#[derive(Debug)]
pub struct CompletionProvider {
    jets_completion: Vec<CompletionItem>,
}

impl CompletionProvider {
    pub fn new() -> Self {
        let jets_completion = Elements::ALL
            .iter()
            .map(|jet| {
                let name = jet.to_string();
                CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some(name.clone()),
                    insert_text: Some(format!(
                        "{}({})",
                        name,
                        jet::source_type(jet.clone())
                            .iter()
                            .enumerate()
                            .map(|(index, item)| { format!("${{{}:{}}}", index + 1, item) })
                            .collect::<Vec<_>>()
                            .join(", ")
                    )),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();

        Self {
            jets_completion: jets_completion,
        }
    }

    pub fn get_jets(&self) -> Vec<CompletionItem> {
        self.jets_completion.to_owned()
    }

    pub fn get_function_completions(&self, functions: Vec<Function>) -> Vec<CompletionItem> {
        functions
            .iter()
            .map(|func| {
                let name = func.name().to_string();
                CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some(name.clone()),
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
            })
            .collect()
    }
}
