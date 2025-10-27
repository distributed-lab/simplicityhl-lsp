use std::borrow::Cow;
use std::fmt::Display;
use std::num::TryFromIntError;

use tower_lsp_server::jsonrpc::Error;
use tower_lsp_server::lsp_types::Uri;

type Message = Cow<'static, str>;

/// Custom error type for LSP server.
#[derive(Debug, Clone)]
pub enum LspError {
    /// An error during the conversion of different types.
    ConversionFailed(Message),

    /// Failed to find function inside `functions` map.
    FunctionNotFound(Message),

    /// Failed to find call inside function.
    CallNotFound(Message),

    /// Failed to find given document inside `documents` map.
    DocumentNotFound(Uri),

    /// A generic or unexpected internal error.
    Internal(Message),
}

impl LspError {
    /// Return error code for error.
    ///
    /// Error code is needed for [`tower_lsp_server::jsonrpc::Error`] to differintiate errors. It's
    /// recommended to use values from 1 to 5000
    pub fn code(&self) -> i64 {
        match self {
            LspError::ConversionFailed(_) => 1,
            LspError::FunctionNotFound(_) => 2,
            LspError::CallNotFound(_) => 3,
            LspError::DocumentNotFound(_) => 4,
            LspError::Internal(_) => 100,
        }
    }

    /// Return description of error.
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

/// Convert [`LspError`] to [`tower_lsp_server::jsonrpc::Error`].
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

/// Convert [`std::num::TryFromIntError`] to [`LspError`].
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
