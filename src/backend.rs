use dashmap::DashMap;
use ropey::Rope;
use serde_json::Value;
use std::str::FromStr;

use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, Diagnostic,
    DidChangeConfigurationParams, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
    DidChangeWorkspaceFoldersParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, ExecuteCommandParams, Hover, HoverParams, HoverProviderCapability,
    InitializeParams, InitializeResult, InitializedParams, MarkupContent, MarkupKind, MessageType,
    OneOf, Range, SaveOptions, SemanticTokensParams, SemanticTokensResult, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions, Uri, WorkDoneProgressOptions, WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
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
    functions_docs: DashMap<String, String>,
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
                hover_provider: Some(HoverProviderCapability::Simple(true)),
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

        let Some(document) = self.document_map.get(uri) else {
            return Ok(Some(CompletionResponse::Array(vec![])));
        };

        let Some(line) = document.text.lines().nth(pos.line as usize) else {
            return Ok(Some(CompletionResponse::Array(vec![])));
        };

        let Some(prefix) = line.slice(..pos.character as usize).as_str() else {
            return Ok(Some(CompletionResponse::Array(vec![])));
        };

        if let Some(last) = prefix
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
        }

        let mut completions =
            CompletionProvider::get_function_completions(document.functions.as_slice());
        completions.extend_from_slice(self.completion_provider.builtins());

        Ok(Some(CompletionResponse::Array(completions)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        Ok(self.provide_hover(&params))
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

        parse_program
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
                if let Some(mut doc) = self.document_map.get_mut(uri) {
                    doc.functions.push(func.to_owned());

                    let rope = doc.text.clone();
                    let start_line =
                        u32::try_from(func.as_ref().start.line.get()).unwrap_or_default() - 1;

                    doc.functions_docs.insert(
                        func.name().to_string(),
                        get_comments_from_lines(start_line, &rope),
                    );
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
                functions_docs: DashMap::new(),
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

    fn provide_hover(&self, params: &HoverParams) -> Option<Hover> {
        let document = self
            .document_map
            .get_mut(&params.text_document_position_params.text_document.uri)?;

        let token_position = params.text_document_position_params.position;
        let token_span = positions_to_span((token_position, token_position)).ok()?;

        let call = find_related_call(&document.functions, token_span)?;
        let (start, end) = span_to_positions(call.span()).ok()?;

        match call.name() {
            parse::CallName::Jet(jet) => {
                let element =
                    simplicityhl::simplicity::jet::Elements::from_str(format!("{jet}").as_str())
                        .ok()?;

                let template = completion::jet::jet_to_template(element);
                let description = format!(
                    "```simplicityhl\nfn jet::{}({}) -> {}\n```\n{}",
                    template.display_name,
                    template.args.join(", "),
                    template.return_type,
                    completion::jet::documentation(element)
                );

                Some(Hover {
                    contents: tower_lsp_server::lsp_types::HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: description,
                    }),
                    range: Some(Range { start, end }),
                })
            }
            parse::CallName::Custom(func) => {
                let function = document.functions.iter().find(|f| f.name() == func)?;
                let function_doc = document.functions_docs.get(&func.to_string())?;

                let template = completion::function_to_template(function);
                let description = format!(
                    "```simplicityhl\nfn {}({}) -> {}\n```\n{}",
                    template.display_name,
                    template.args.join(", "),
                    template.return_type,
                    *function_doc
                );
                Some(Hover {
                    contents: tower_lsp_server::lsp_types::HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: description,
                    }),
                    range: Some(Range { start, end }),
                })
            }
            _ => None,
        }
    }
}

fn get_comments_from_lines(line: u32, rope: &Rope) -> String {
    let mut result = vec![];

    if line == 0 {
        return String::new();
    }
    for i in (0..line).rev() {
        let Some(rope_slice) = rope.get_line(i as usize) else {
            break;
        };
        let text = rope_slice.to_string();

        if text.starts_with("///") {
            let doc = text.strip_prefix("///").unwrap_or("").to_string();
            result.push(doc);
        } else {
            break;
        }
    }

    result.reverse();
    result.join("\n")
}

fn find_related_call(
    functions: &[parse::Function],
    token_span: simplicityhl::error::Span,
) -> Option<&simplicityhl::parse::Call> {
    let func = functions
        .iter()
        .find(|func| span_contains(func.span(), &token_span))?;

    let calls = parse::ExprTree::Expression(func.body())
        .pre_order_iter()
        .filter_map(|expr| match expr {
            parse::ExprTree::Call(call) => Some(call),
            _ => None,
        })
        .collect::<Vec<_>>();

    calls
        .iter()
        .copied()
        .filter(|s| span_contains(s.span(), &token_span))
        .next_back()
}
