use dashmap::DashMap;
use ropey::Rope;
use serde_json::Value;

use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::lsp_types::*;
use tower_lsp_server::{Client, LanguageServer, LspService, Server};

use simplicityhl::{
    CompiledProgram, ast,
    error::{RichError, Span, WithFile},
    parse,
    parse::ParseFromStr,
};

mod symbol_table;

#[derive(Debug)]
struct Backend {
    client: Client,
    document_map: DashMap<String, Rope>,
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
                    trigger_characters: Some(vec![".".to_string()]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    ..Default::default()
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["dummy.do_something".to_string()],
                    work_done_progress_options: Default::default(),
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
        self.client
            .log_message(MessageType::INFO, "initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
        self.client
            .log_message(MessageType::INFO, "workspace folders changed!")
            .await;
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
        self.client
            .log_message(MessageType::INFO, "configuration changed!")
            .await;
    }

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        self.client
            .log_message(MessageType::INFO, "watched files have changed!")
            .await;
    }

    async fn execute_command(&self, _: ExecuteCommandParams) -> Result<Option<Value>> {
        self.client
            .log_message(MessageType::INFO, "command executed!")
            .await;

        match self.client.apply_edit(WorkspaceEdit::default()).await {
            Ok(res) if res.applied => self.client.log_message(MessageType::INFO, "applied").await,
            Ok(_) => self.client.log_message(MessageType::INFO, "rejected").await,
            Err(err) => self.client.log_message(MessageType::ERROR, err).await,
        }

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
        self.client
            .log_message(MessageType::INFO, "file saved!")
            .await;
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file closed!")
            .await;
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
            CompletionItem::new_simple("Bye".to_string(), "More detail".to_string()),
        ])))
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

impl Backend {
    fn parse_program(text: &str) -> Option<RichError> {
        let parse_program = match parse::Program::parse_from_str(text) {
            Ok(p) => p,
            Err(e) => return Some(e),
        };

        match ast::Program::analyze(&parse_program).with_file(text) {
            Ok(ast) => None,
            Err(e) => Some(e),
        }
    }

    async fn on_change<'a>(&self, params: TextDocumentItem<'a>) {
        let rope = ropey::Rope::from_str(params.text);
        self.document_map
            .insert(params.uri.to_string(), rope.clone());

        let err = Backend::parse_program(&params.text);

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

#[tokio::main]
async fn main() {
    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());

    let (service, socket) = LspService::new(|client| Backend {
        client: client,
        document_map: DashMap::new(),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
