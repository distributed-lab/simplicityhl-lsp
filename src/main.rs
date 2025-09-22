use std::fmt::Write;

use dashmap::DashMap;
use ropey::Rope;
use serde_json::Value;

use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::lsp_types::*;
use tower_lsp_server::{Client, LanguageServer, LspService, Server};

use tree_sitter::{self, StreamingIterator};
use tree_sitter_simfony;

use simplicityhl::{
    ast,
    error::{RichError, Span, WithFile},
    parse,
    parse::ParseFromStr,
};

#[derive(Debug)]
struct Backend {
    client: Client,
    document_map: DashMap<String, Rope>,
    token_legend: Vec<SemanticTokenType>,
    token_map: DashMap<String, u32>,
}

struct TextDocumentItem<'a> {
    uri: Uri,
    text: &'a str,
    version: Option<i32>,
}

fn build_token_map(legend: &[SemanticTokenType]) -> DashMap<String, u32> {
    legend
        .iter()
        .enumerate()
        .map(|(i, t)| (t.as_str().to_string(), i as u32))
        .collect()
}

fn get_token_capabilities(
    token_types: &Vec<SemanticTokenType>,
) -> SemanticTokensServerCapabilities {
    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
        legend: SemanticTokensLegend {
            token_types: token_types.clone(),
            token_modifiers: vec![],
        },
        full: Some(SemanticTokensFullOptions::Bool(true)),
        range: None,
        ..Default::default()
    })
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
                semantic_tokens_provider: Some(get_token_capabilities(&self.token_legend)),
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

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let text = match self.document_map.get(uri.as_str()) {
            Some(rope) => rope.to_string(),
            None => "".to_string(),
        };
        let tokens = self.highlight_with_treesitter(&text).await;

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens,
        })))
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
    async fn highlight_with_treesitter(&self, code: &str) -> Vec<SemanticToken> {
        let mut parser = tree_sitter::Parser::new();
        let language = tree_sitter_simfony::LANGUAGE;

        parser
            .set_language(&language.into())
            .expect("Error loading Simfony parser");
        let tree = parser.parse(code, None).unwrap();

        let query = tree_sitter::Query::new(&language.into(), include_str!("highlights.scm"))
            .expect("file should open and be valid");
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut tokens = Vec::new();

        let (mut last_line, mut last_col) = (0, 0);

        cursor
            .matches(&query, tree.root_node(), code.as_bytes())
            .for_each(|m| {
                for cap in m.captures {
                    let node = cap.node;
                    let (line, col) = (
                        node.start_position().row as u32,
                        node.start_position().column as u32,
                    );
                    let (delta_line, delta_start) = if line == last_line {
                        (0, col - last_col)
                    } else {
                        (line - last_line, col)
                    };

                    let length = node.end_byte() - node.start_byte();
                    let kind = query.capture_names()[cap.index as usize];
                    let token_type_index = self.token_map.get(kind);

                    match token_type_index {
                        Some(index) => {
                            tokens.push(SemanticToken {
                                delta_line: delta_line,
                                delta_start: delta_start,
                                length: length as u32,
                                token_type: *index,
                                token_modifiers_bitset: 0,
                            });

                            (last_line, last_col) = (line, col);
                        }
                        None => {}
                    }
                }
            });
        self.client
            .log_message(MessageType::LOG, "Done with semantic highlight!")
            .await;

        tokens
    }

    fn parse_program(text: &str) -> Option<RichError> {
        let parse_program = match parse::Program::parse_from_str(text) {
            Ok(p) => p,
            Err(e) => return Some(e),
        };

        match ast::Program::analyze(&parse_program).with_file(text) {
            Ok(_ast) => None,
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

    let legend: Vec<SemanticTokenType> = vec![
        "function".into(),
        "variable".into(),
        "keyword".into(),
        "type".into(),
        "parameter".into(),
        "comment".into(),
        "number".into(),
        "operator".into(),
    ];

    let (service, socket) = LspService::new(|client| Backend {
        client: client,
        document_map: DashMap::new(),
        token_map: build_token_map(&legend),
        token_legend: legend,
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
