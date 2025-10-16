use ropey::Rope;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, Diagnostic,
    DidChangeConfigurationParams, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
    DidChangeWorkspaceFoldersParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, ExecuteCommandParams, GotoDefinitionParams, GotoDefinitionResponse,
    Hover, HoverParams, HoverProviderCapability, InitializeParams, InitializeResult,
    InitializedParams, Location, MarkupContent, MarkupKind, MessageType, OneOf, Range, SaveOptions,
    SemanticTokensParams, SemanticTokensResult, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, TextDocumentSyncSaveOptions, Uri,
    WorkDoneProgressOptions, WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
};
use tower_lsp_server::{Client, LanguageServer};

use simplicityhl::{
    ast,
    error::{RichError, WithFile},
    parse,
    parse::ParseFromStr,
};

use miniscript::iter::TreeLike;

use crate::completion::{self, CompletionProvider};
use crate::utils::{positions_to_span, span_contains, span_to_positions};

#[derive(Debug)]
struct Document {
    functions: Vec<parse::Function>,
    functions_docs: HashMap<String, String>,
    text: Rope,
}

#[derive(Debug)]
pub struct Backend {
    client: Client,

    document_map: Arc<RwLock<HashMap<Uri, Document>>>,

    completion_provider: CompletionProvider,
}

struct TextDocumentItem<'a> {
    uri: Uri,
    text: &'a str,
    version: Option<i32>,
}

impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(true),
                        })),
                        ..Default::default()
                    },
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![":".to_string()]),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {}

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {}

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {}

    async fn execute_command(&self, _: ExecuteCommandParams) -> Result<Option<Value>> {
        Ok(None)
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: &params.text_document.text,
            version: Some(params.text_document.version),
        })
        .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.on_change(TextDocumentItem {
            text: &params.content_changes[0].text,
            uri: params.text_document.uri,
            version: Some(params.text_document.version),
        })
        .await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            self.on_change(TextDocumentItem {
                uri: params.text_document.uri,
                text: &text,
                version: None,
            })
            .await;
        }
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {}

    async fn semantic_tokens_full(
        &self,
        _: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let documents = self.document_map.read().await;

        let Some(doc) = documents.get(uri) else {
            return Ok(Some(CompletionResponse::Array(vec![])));
        };

        let Some(line) = doc.text.lines().nth(pos.line as usize) else {
            return Ok(Some(CompletionResponse::Array(vec![])));
        };

        let Some(slice) = line.get_slice(..pos.character as usize) else {
            return Ok(Some(CompletionResponse::Array(vec![])));
        };

        let Some(prefix) = slice.as_str() else {
            return Ok(Some(CompletionResponse::Array(vec![])));
        };

        let trimmed_prefix = prefix.trim_end();

        if let Some(last) = trimmed_prefix
            .rsplit(|c: char| !c.is_alphanumeric() && c != ':')
            .next()
        {
            if last.starts_with("jet:::") {
                return Ok(Some(CompletionResponse::Array(vec![])));
            } else if last == "jet::" || last.starts_with("jet::") {
                return Ok(Some(CompletionResponse::Array(
                    self.completion_provider.jets().to_vec(),
                )));
            }
        // completion after colon needed only for jets
        } else if trimmed_prefix.ends_with(':') {
            return Ok(Some(CompletionResponse::Array(vec![])));
        }

        let mut completions = CompletionProvider::get_function_completions(
            &doc.functions
                .iter()
                .map(|func| {
                    let function_doc = doc
                        .functions_docs
                        .get(&func.name().to_string())
                        .map_or(String::new(), String::clone);
                    (func.to_owned(), function_doc)
                })
                .collect::<Vec<_>>(),
        );
        completions.extend_from_slice(self.completion_provider.builtins());
        completions.extend_from_slice(self.completion_provider.modules());

        Ok(Some(CompletionResponse::Array(completions)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        Ok(self.provide_hover(&params).await)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let documents = self.document_map.read().await;
        let uri = &params.text_document_position_params.text_document.uri;

        let result = || -> Option<GotoDefinitionResponse> {
            let document = documents.get(uri)?;

            let token_position = params.text_document_position_params.position;
            let token_span = positions_to_span((token_position, token_position)).ok()?;

            let call = find_related_call(&document.functions, token_span)?;

            match call.name() {
                simplicityhl::parse::CallName::Custom(func) => {
                    let function = document
                        .functions
                        .iter()
                        .find(|function| function.name() == func)?;

                    let (start, end) = span_to_positions(function.as_ref()).ok()?;
                    Some(GotoDefinitionResponse::from(Location::new(
                        uri.clone(),
                        Range::new(start, end),
                    )))
                }
                _ => None,
            }
        }();

        Ok(result)
    }
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            document_map: Arc::new(RwLock::new(HashMap::new())),
            completion_provider: CompletionProvider::new(),
        }
    }

    /// Function which executed on change of file (`did_save`, `did_open` or `did_change` methods)
    async fn on_change(&self, params: TextDocumentItem<'_>) {
        let (err, document) = parse_program(params.text);

        let mut documents = self.document_map.write().await;
        if let Some(doc) = document {
            documents.insert(params.uri.clone(), doc);
        } else if let Some(doc) = documents.get_mut(&params.uri) {
            doc.text = Rope::from_str(params.text);
        }

        match err {
            None => {
                self.client
                    .publish_diagnostics(params.uri.clone(), vec![], params.version)
                    .await;
            }
            Some(err) => {
                let (start, end) = match span_to_positions(err.span()) {
                    Ok(result) => result,
                    Err(err) => {
                        self.client
                            .log_message(
                                MessageType::ERROR,
                                format!("Catch error while parsing span: {err}"),
                            )
                            .await;
                        return;
                    }
                };

                self.client
                    .publish_diagnostics(
                        params.uri.clone(),
                        vec![Diagnostic::new_simple(
                            Range::new(start, end),
                            err.error().to_string(),
                        )],
                        params.version,
                    )
                    .await;
            }
        }
    }

    /// Provide hover for [`Backend::hover`] function.
    async fn provide_hover(&self, params: &HoverParams) -> Option<Hover> {
        let documents = self.document_map.read().await;

        let document = documents.get(&params.text_document_position_params.text_document.uri)?;

        let token_position = params.text_document_position_params.position;
        let token_span = positions_to_span((token_position, token_position)).ok()?;

        let call = find_related_call(&document.functions, token_span)?;
        let (start, end) = span_to_positions(call.span()).ok()?;

        let description = match call.name() {
            parse::CallName::Jet(jet) => {
                let element =
                    simplicityhl::simplicity::jet::Elements::from_str(format!("{jet}").as_str())
                        .ok()?;

                let template = completion::jet::jet_to_template(element);
                format!(
                    "```simplicityhl\nfn jet::{}({}) -> {}\n```\n{}",
                    template.display_name,
                    template.args.join(", "),
                    template.return_type,
                    template.description
                )
            }
            parse::CallName::Custom(func) => {
                let function = document.functions.iter().find(|f| f.name() == func)?;
                let function_doc = document.functions_docs.get(&func.to_string())?;

                let template = completion::function_to_template(function, function_doc);
                format!(
                    "```simplicityhl\nfn {}({}) -> {}\n```\n{}",
                    template.display_name,
                    template.args.join(", "),
                    template.return_type,
                    template.description
                )
            }
            other => {
                let template = completion::builtin::match_callname(other)?;
                format!(
                    "```simplicityhl\nfn {}({}) -> {}\n```\n{}",
                    template.display_name,
                    template.args.join(", "),
                    template.return_type,
                    template.description
                )
            }
        };

        Some(Hover {
            contents: tower_lsp_server::lsp_types::HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: description,
            }),
            range: Some(Range { start, end }),
        })
    }
}

/// Create [`Document`] using parsed program and code.
fn create_document(program: &simplicityhl::parse::Program, text: &str) -> Document {
    let mut document = Document {
        functions: vec![],
        functions_docs: HashMap::new(),
        text: Rope::from_str(text),
    };

    program
        .items()
        .iter()
        .filter_map(|item| {
            if let parse::Item::Function(func) = item {
                Some(func)
            } else {
                None
            }
        })
        .for_each(|func| {
            let start_line = u32::try_from(func.as_ref().start.line.get()).unwrap_or_default() - 1;

            document.functions.push(func.to_owned());
            document.functions_docs.insert(
                func.name().to_string(),
                get_comments_from_lines(start_line, &document.text),
            );
        });

    document
}

/// Parse program using [`simplicityhl`] compiler and return [`RichError`],
/// which used in Diagnostic. Also create [`Document`] from parsed program.
fn parse_program(text: &str) -> (Option<RichError>, Option<Document>) {
    let program = match parse::Program::parse_from_str(text) {
        Ok(p) => p,
        Err(e) => return (Some(e), None),
    };

    (
        ast::Program::analyze(&program).with_file(text).err(),
        Some(create_document(&program, text)),
    )
}

/// Get document comments, using lines above given line index. Only used to
/// get documentation for custom functions.
fn get_comments_from_lines(line: u32, rope: &Rope) -> String {
    let mut lines = Vec::new();

    if line == 0 {
        return String::new();
    }

    for i in (0..line).rev() {
        let Some(rope_slice) = rope.get_line(i as usize) else {
            break;
        };
        let text = rope_slice.to_string();

        if text.starts_with("///") {
            let doc = text
                .strip_prefix("///")
                .unwrap_or("")
                .trim_end()
                .to_string();
            lines.push(doc);
        } else {
            break;
        }
    }

    lines.reverse();

    let mut result = String::new();
    let mut prev_line_was_text = false;

    for line in lines {
        let trimmed = line.trim();

        let is_md_block = trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with('-')
            || trimmed.starts_with('*')
            || trimmed.starts_with('>')
            || trimmed.starts_with("```")
            || trimmed.starts_with("    ");

        if result.is_empty() {
            result.push_str(trimmed);
        } else if prev_line_was_text && !is_md_block {
            result.push(' ');
            result.push_str(trimmed);
        } else {
            result.push('\n');
            result.push_str(trimmed);
        }

        prev_line_was_text = !trimmed.is_empty() && !is_md_block;
    }

    result
}

/// Find [`simplicityhl::parse::Call`] which contains given [`simplicityhl::error::Span`], which also have minimal Span.
fn find_related_call(
    functions: &[parse::Function],
    token_span: simplicityhl::error::Span,
) -> Option<&simplicityhl::parse::Call> {
    let func = functions
        .iter()
        .find(|func| span_contains(func.span(), &token_span))?;

    parse::ExprTree::Expression(func.body())
        .pre_order_iter()
        .filter_map(|expr| {
            if let parse::ExprTree::Call(call) = expr {
                Some(call)
            } else {
                None
            }
        })
        .filter(|c| span_contains(c.span(), &token_span))
        .last()
}
