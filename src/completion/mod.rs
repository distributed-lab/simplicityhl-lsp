use simplicityhl::parse::Function;

pub mod builtin;
pub mod jet;
pub mod types;

use tower_lsp_server::lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
};

/// Build and provide `CompletionItem` for Jets and builtin functions.
#[derive(Debug)]
pub struct CompletionProvider {
    /// All jets completions.
    jets_completion: Vec<CompletionItem>,

    /// All builtin functions completions.
    builtin_completion: Vec<CompletionItem>,
}

impl CompletionProvider {
    /// Create new `CompletionProvider` with evaluated jets and builtins completions.
    pub fn new() -> Self {
        let jets_completion = jet::get_jets_completions()
            .iter()
            .map(template_to_completion)
            .collect();
        let builtin_completion = builtin::get_builtin_functions()
            .iter()
            .map(template_to_completion)
            .collect();

        Self {
            jets_completion,
            builtin_completion,
        }
    }

    /// Return jets completions.
    pub fn jets(&self) -> &[CompletionItem] {
        &self.jets_completion
    }

    /// Return builtin functions completions.
    pub fn builtins(&self) -> &[CompletionItem] {
        &self.builtin_completion
    }

    /// Get generic functions completions.
    pub fn get_function_completions(functions: &[(Function, String)]) -> Vec<CompletionItem> {
        functions
            .iter()
            .map(|(func, doc)| {
                let template = function_to_template(func, doc);
                template_to_completion(&template)
            })
            .collect()
    }
}

/// Convert `simplicityhl::parse::Function` to `FunctionTemplate`.
pub fn function_to_template(func: &Function, doc: &String) -> types::FunctionTemplate {
    types::FunctionTemplate::simple(
        func.name().to_string(),
        func.params().iter().map(|item| format!("{item}")).collect(),
        match func.ret() {
            Some(ret) => format!("{ret}"),
            None => "()".to_string(),
        },
        doc,
    )
}

/// Convert `FunctionCompletionTemplate` to `CompletionItem`.
fn template_to_completion(func: &types::FunctionTemplate) -> CompletionItem {
    CompletionItem {
        label: func.display_name.clone(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some(func.get_signature()),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: func.description.clone(),
        })),
        insert_text: Some(func.get_insert_text()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
    }
}
