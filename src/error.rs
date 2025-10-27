use std::borrow::Cow;
use std::fmt::Display;
use std::num::TryFromIntError;

use tower_lsp_server::jsonrpc::Error;
use tower_lsp_server::lsp_types::Uri;

type Message = Cow<'static, str>;

/// The main error type for our language server.
#[derive(Debug, Clone)]
pub enum LspError {
    /// An error during the conversion between LSP positions and compiler spans.
    ConversionFailed(Message),

    /// Failed to find a relevant item in the AST for a request.
    FunctionNotFound(Message),

    /// Call not found inside function.
    CallNotFound(Message),

    /// The requested document URI was not found in the server's state.
    DocumentNotFound(Uri),

    /// A generic or unexpected internal error.
    Internal(Message),
}

impl LspError {
    pub fn code(&self) -> i64 {
        match self {
            LspError::ConversionFailed(_) => 1,
            LspError::FunctionNotFound(_) => 2,
            LspError::CallNotFound(_) => 3,
            LspError::DocumentNotFound(_) => 4,
            LspError::Internal(_) => 100,
        }
    }

    pub fn description(&self) -> String {
        match self {
            LspError::DocumentNotFound(uri) => {
                format!("Document not found: {}", uri.as_str())
            }
            LspError::ConversionFailed(cow)
            | LspError::FunctionNotFound(cow)
            | LspError::CallNotFound(cow)
            | LspError::Internal(cow) => cow.to_string(),
        }
    }
}

impl From<LspError> for Error {
    fn from(err: LspError) -> Self {
        let code = err.code();
        let msg = err.description();

        Error {
            code: code.into(),
            message: msg.into(),
            data: None,
        }
    }
}

impl From<TryFromIntError> for LspError {
    fn from(value: TryFromIntError) -> Self {
        LspError::ConversionFailed(value.to_string().into())
    }
}

impl Display for LspError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{}: {}", self.code(), self.description()).as_str())
    }
}
