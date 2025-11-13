use simplicityhl::parse::Function;

pub mod builtin;
pub mod jet;
pub mod type_cast;
pub mod types;

use tower_lsp_server::lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
};

/// Build and provide [`CompletionItem`] for jets and builtin functions.
#[derive(Debug)]
pub struct CompletionProvider {
    /// All jets completions.
    jets: Vec<CompletionItem>,

    /// All builtin functions completions.
    builtin: Vec<CompletionItem>,

    /// Modules completions.
    modules: Vec<CompletionItem>,

    /// Default Type cast completions.
    type_casts: Vec<CompletionItem>,
}

impl CompletionProvider {
    /// Create new [`CompletionProvider`] with evaluated jets and builtins completions.
    pub fn new() -> Self {
        let jets_completion = jet::get_jets_completions()
            .iter()
            .map(template_to_completion)
            .collect();
        let builtin_completion = builtin::get_builtin_functions()
            .iter()
            .map(template_to_completion)
            .collect();

        let modules_completion = [
            ("jet", "Module which contains jets"),
            ("param", "Module which contains parameters"),
            ("witness", "Module which contains witnesses"),
        ]
        .iter()
        .map(|(module, detail)| module_to_completion((*module).to_string(), (*detail).to_string()))
        .collect();

        let type_casts_completion = type_cast::get_integer_type_casts()
        let type_casts_completion = type_cast::TYPE_CASTS
            .iter()
            .map(|(&to, &from)| CompletionItem {
                label: format!("<{from}>::into({from})"),
                label: format!("{to} <- {from}"),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(format!("Cast into type `{to}`",)),
                documentation: None,
                insert_text: Some(format!("{from}>::into(${{1:{from}}})")),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            })
            .collect::<Vec<_>>();

        Self {
            jets: jets_completion,
            builtin: builtin_completion,
            modules: modules_completion,
            type_casts: type_casts_completion,
        }
    }

    /// Get generic functions completions.
    pub fn get_function_completions(functions: &[(&Function, &str)]) -> Vec<CompletionItem> {
        functions
            .iter()
            .map(|(func, doc)| {
                let template = function_to_template(func, doc);
                template_to_completion(&template)
            })
            .collect()
    }

    /// Return completions based on line and functions provided.
    pub fn process_completions(
        &self,
        prefix: &str,
        functions: &[(&Function, &str)],
    ) -> Option<Vec<CompletionItem>> {
        if let Some(last) = prefix
            .rsplit(|c: char| !c.is_alphanumeric() && c != ':' && c != '<')
            .next()
        {
            if last.starts_with("jet::") {
                return Some(self.jets.clone());
            }
            if last.starts_with('<') {
                return Some(self.type_casts.clone());
            }
        }

        if prefix.ends_with(':') {
            return None;
        }
        let mut completions = CompletionProvider::get_function_completions(functions);
        // return only function completions in case of '<' symbol in `for_while`, `array_fold` and
        // `fold`
        if prefix.ends_with('<') {
            return Some(completions);
        }

        completions.extend_from_slice(&self.builtin);
        completions.extend_from_slice(&self.modules);

        Some(completions)
    }
}

/// Convert [`simplicityhl::parse::Function`] to [`types::FunctionTemplate`].
pub fn function_to_template(func: &Function, doc: &str) -> types::FunctionTemplate {
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

/// Convert [`types::FunctionTemplate`] to [`CompletionItem`].
fn template_to_completion(func: &types::FunctionTemplate) -> CompletionItem {
    CompletionItem {
        label: func.display_name.clone(),
        // Because `into` has different structure, completion with CompletionItemKind::FUNCTION
        // have strange visual effects, so we use CompletionItemKind::SNIPPET
        kind: Some(if func.display_name == "into" {
            CompletionItemKind::SNIPPET
        } else {
            CompletionItemKind::FUNCTION
        }),
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

/// Convert module name to [`CompletionItem`].
fn module_to_completion(module: String, detail: String) -> CompletionItem {
    CompletionItem {
        label: module.clone(),
        kind: Some(CompletionItemKind::MODULE),
        detail: Some(detail),
        documentation: None,
        insert_text: Some(module),
        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
        ..Default::default()
    }
}
