use simplicityhl::error::{Position, Span};

fn position_le(a: &Position, b: &Position) -> bool {
    (a.line < b.line) || (a.line == b.line && a.col <= b.col)
}

fn position_ge(a: &Position, b: &Position) -> bool {
    (a.line > b.line) || (a.line == b.line && a.col >= b.col)
}

pub fn span_contains(a: &Span, b: &Span) -> bool {
    position_le(&a.start, &b.start) && position_ge(&a.end, &b.end)
}
