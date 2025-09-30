use dashmap::DashMap;
use ropey::Rope;
use serde_json::Value;

use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::lsp_types::*;
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

    document_map: DashMap<String, Document>,

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
                    work_done_progress_options: Default::default(),
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
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        debug!("server initialized!")
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
        .await
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.on_change(TextDocumentItem {
            text: &params.content_changes[0].text,
            uri: params.text_document.uri,
            version: Some(params.text_document.version),
        })
        .await
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        match params.text {
            Some(text) => {
                self.on_change(TextDocumentItem {
                    uri: params.text_document.uri,
                    text: &text,
                    version: None,
                })
                .await;
            }
            None => {}
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
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let map = self.document_map.get(&uri.to_string());
        debug!("completion");
        match map {
            Some(document) => {
                let line = document.text.lines().nth(pos.line as usize).unwrap();
                let prefix = &line.slice(..pos.character as usize);

                if prefix.as_str().unwrap().ends_with("jet::") {
                    return Ok(Some(CompletionResponse::Array(
                        self.completion_provider.get_jets(),
                    )));
                } else {
                    return Ok(Some(CompletionResponse::Array(
                        self.completion_provider
                            .get_function_completions(document.functions.to_owned()),
                    )));
                }
            }
            None => {}
        }
        Ok(Some(CompletionResponse::Array(vec![])))
    }
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client: client,
            document_map: DashMap::new(),
            completion_provider: CompletionProvider::new(),
        }
    }

    fn parse_program(&self, text: &str, uri: &String) -> Option<RichError> {
        let parse_program = match parse::Program::parse_from_str(text) {
            Ok(p) => p,
            Err(e) => return Some(e),
        };

        parse_program.items().iter().for_each(|item| match item {
            parse::Item::Function(func) => {
                self.document_map
                    .get_mut(uri)
                    .unwrap()
                    .functions
                    .push(func.to_owned());
            }
            _ => {}
        });

        match ast::Program::analyze(&parse_program).with_file(text) {
            Ok(_ast) => None,
            Err(e) => Some(e),
        }
    }

    async fn on_change<'a>(&self, params: TextDocumentItem<'a>) {
        let rope = ropey::Rope::from_str(params.text);
        self.document_map.insert(
            params.uri.to_string(),
            Document {
                functions: vec![],
                text: rope.clone(),
            },
        );

        let err = self.parse_program(&params.text, &params.uri.to_string());

        match err {
            None => {
                self.client
                    .log_message(MessageType::INFO, &format!("errors not found!"))
                    .await;
                self.client
                    .publish_diagnostics(params.uri.clone(), vec![], params.version)
                    .await;
            }
            Some(err) => {
                let (start, end) = span_to_positions(err.span());

                self.client
                    .log_message(MessageType::INFO, &format!("Get error: \n{}", err))
                    .await;

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

fn span_to_positions(span: &Span) -> (Position, Position) {
    (
        Position {
            line: span.start.line.get() as u32 - 1,
            character: span.start.col.get() as u32 - 1,
        },
        Position {
            line: span.end.line.get() as u32 - 1,
            character: span.end.col.get() as u32 - 1,
        },
    )
}
