use std::num::NonZeroUsize;

use tower_lsp_server::jsonrpc::{Error, Result};
use tower_lsp_server::lsp_types;

fn position_le(a: &simplicityhl::error::Position, b: &simplicityhl::error::Position) -> bool {
    (a.line < b.line) || (a.line == b.line && a.col <= b.col)
}

fn position_ge(a: &simplicityhl::error::Position, b: &simplicityhl::error::Position) -> bool {
    (a.line > b.line) || (a.line == b.line && a.col >= b.col)
}

pub fn span_contains(a: &simplicityhl::error::Span, b: &simplicityhl::error::Span) -> bool {
    position_le(&a.start, &b.start) && position_ge(&a.end, &b.end)
}

/// Convert [`simplicityhl::error::Span`] to [`tower_lsp_server::lsp_types::Position`]
///
/// Converting is required because `simplicityhl::error::Span` using their own versions of `Position`,
/// which contains non-zero column and line, so they are always starts with one.
/// `Position` required for diagnostic starts with zero
pub fn span_to_positions(
    span: &simplicityhl::error::Span,
) -> Result<(lsp_types::Position, lsp_types::Position)> {
    let start_line = u32::try_from(span.start.line.get())
        .map_err(|e| Error::invalid_params(format!("line overflow: {e}")))?;
    let start_col = u32::try_from(span.start.col.get())
        .map_err(|e| Error::invalid_params(format!("col overflow: {e}")))?;
    let end_line = u32::try_from(span.end.line.get())
        .map_err(|e| Error::invalid_params(format!("line overflow: {e}")))?;
    let end_col = u32::try_from(span.end.col.get())
        .map_err(|e| Error::invalid_params(format!("col overflow: {e}")))?;

    Ok((
        lsp_types::Position {
            line: start_line - 1,
            character: start_col - 1,
        },
        lsp_types::Position {
            line: end_line - 1,
            character: end_col - 1,
        },
    ))
}

#[allow(dead_code)]
/// Convert pair of [`tower_lsp_server::lsp_types::Position`] to [`simplicityhl::error::Span`]
pub fn positions_to_span(
    positions: (lsp_types::Position, lsp_types::Position),
) -> Result<simplicityhl::error::Span> {
    let start_line = NonZeroUsize::new((positions.0.line + 1) as usize)
        .ok_or_else(|| Error::invalid_params("start line must be non-zero".to_string()))?;

    let start_col = NonZeroUsize::new((positions.0.character + 1) as usize)
        .ok_or_else(|| Error::invalid_params("start column must be non-zero".to_string()))?;

    let end_line = NonZeroUsize::new((positions.1.line + 1) as usize)
        .ok_or_else(|| Error::invalid_params("end line must be non-zero".to_string()))?;

    let end_col = NonZeroUsize::new((positions.1.character + 1) as usize)
        .ok_or_else(|| Error::invalid_params("end column must be non-zero".to_string()))?;
    Ok(simplicityhl::error::Span {
        start: simplicityhl::error::Position {
            line: start_line,
            col: start_col,
        },
        end: simplicityhl::error::Position {
            line: end_line,
            col: end_col,
        },
    })
}

/// Convert [`tower_lsp_server::lsp_types::Position`] to [`simplicityhl::error::Span`]
///
/// Useful when [`tower_lsp_server::lsp_types::Position`] represents some singular point.
pub fn position_to_span(position: lsp_types::Position) -> Result<simplicityhl::error::Span> {
    let start_line = NonZeroUsize::new((position.line + 1) as usize)
        .ok_or_else(|| Error::invalid_params("start line must be non-zero".to_string()))?;

    let start_col = NonZeroUsize::new((position.character + 1) as usize)
        .ok_or_else(|| Error::invalid_params("start column must be non-zero".to_string()))?;

    Ok(simplicityhl::error::Span {
        start: simplicityhl::error::Position {
            line: start_line,
            col: start_col,
        },
        end: simplicityhl::error::Position {
            line: start_line,
            col: start_col,
        },
    })
}
