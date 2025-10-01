use dashmap::DashMap;
use ropey::Rope;
use serde_json::Value;

use tower_lsp_server::jsonrpc::{Error, Result};
use tower_lsp_server::lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, Diagnostic,
    DidChangeConfigurationParams, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
    DidChangeWorkspaceFoldersParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, ExecuteCommandParams, InitializeParams, InitializeResult,
    InitializedParams, MessageType, OneOf, Position, Range, SaveOptions, SemanticTokensParams,
    SemanticTokensResult, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextDocumentSyncSaveOptions, Uri, WorkDoneProgressOptions,
    WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
};
use tower_lsp_server::{Client, LanguageServer};

use log::debug;
use simplicityhl::{
    ast,
    error::{RichError, Span, WithFile},
    parse,
    parse::ParseFromStr,
};

use crate::completion::CompletionProvider;

#[derive(Debug)]
struct Document {
    functions: Vec<parse::Function>,
    text: Rope,
}

#[derive(Debug)]
pub struct Backend {
    client: Client,

    document_map: DashMap<Uri, Document>,

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
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        debug!("server initialized!");
    }

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

        debug!("saved!");
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {
        debug!("closed!");
    }

    async fn semantic_tokens_full(
        &self,
        _: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;

        let Some(document) = self.document_map.get(uri) else {
            return Ok(Some(CompletionResponse::Array(vec![])));
        };

        let Some(line) = document.text.lines().nth(pos.line as usize) else {
            return Ok(Some(CompletionResponse::Array(vec![])));
        };

        let Some(prefix) = line.slice(..pos.character as usize).as_str() else {
            return Ok(Some(CompletionResponse::Array(vec![])));
        };

        if prefix.ends_with("jet::") {
            return Ok(Some(CompletionResponse::Array(
                self.completion_provider.jets().to_vec(),
            )));
        }

        Ok(Some(CompletionResponse::Array(
            CompletionProvider::get_function_completions(document.functions.as_slice()),
        )))
    }
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            document_map: DashMap::new(),
            completion_provider: CompletionProvider::new(),
        }
    }

    fn parse_program(&self, text: &str, uri: &Uri) -> Option<RichError> {
        let parse_program = match parse::Program::parse_from_str(text) {
            Ok(p) => p,
            Err(e) => return Some(e),
        };

        parse_program.items().iter().for_each(|item| {
            if let parse::Item::Function(func) = item {
                self.document_map
                    .get_mut(uri)
                    // TODO: avoid unwraps at all cost
                    .unwrap()
                    .functions
                    .push(func.to_owned());
            }
        });

        ast::Program::analyze(&parse_program).with_file(text).err()
    }

    async fn on_change(&self, params: TextDocumentItem<'_>) {
        let rope = ropey::Rope::from_str(params.text);
        self.document_map.insert(
            params.uri.clone(),
            Document {
                functions: vec![],
                text: rope.clone(),
            },
        );

        let err = self.parse_program(params.text, &params.uri);

        match err {
            None => {
                self.client
                    .log_message(MessageType::INFO, "errors not found!".to_string())
                    .await;
                self.client
                    .publish_diagnostics(params.uri.clone(), vec![], params.version)
                    .await;
            }
            Some(err) => {
                let (start, end) = match span_to_positions(err.span()) {
                    Ok(result) => result,
                    Err(err) => {
                        dbg!("catch error: {}", err);
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
}

/// Convert `simplicityhl::error::Span` to `tower_lsp_server::lsp_types::Positions`
///
/// Converting is required because `simplicityhl::error::Span` using their own versions of `Position`,
/// which contains non-zero column and line, so they are always starts with one.
/// `Position` required for diagnostic starts with zero
fn span_to_positions(span: &Span) -> Result<(Position, Position)> {
    let start_line = u32::try_from(span.start.line.get())
        .map_err(|e| Error::invalid_params(format!("line overflow: {e}")))?;
    let start_col = u32::try_from(span.start.col.get())
        .map_err(|e| Error::invalid_params(format!("col overflow: {e}")))?;
    let end_line = u32::try_from(span.end.line.get())
        .map_err(|e| Error::invalid_params(format!("line overflow: {e}")))?;
    let end_col = u32::try_from(span.end.col.get())
        .map_err(|e| Error::invalid_params(format!("col overflow: {e}")))?;

    Ok((
        Position {
            line: start_line - 1,
            character: start_col - 1,
        },
        Position {
            line: end_line - 1,
            character: end_col - 1,
        },
    ))
}
