use dashmap::DashMap;
use tower_lsp_server::lsp_types::{SemanticToken, SemanticTokenType};
use tree_sitter::{self, StreamingIterator};
use tree_sitter_simfony;

fn build_token_map(legend: &[SemanticTokenType]) -> DashMap<String, u32> {
    legend
        .iter()
        .enumerate()
        .map(|(i, t)| (t.as_str().to_string(), i as u32))
        .collect()
}

#[derive(Debug)]
pub struct TokenProvider {
    token_legend: Vec<SemanticTokenType>,
    token_map: DashMap<String, u32>,
}

impl TokenProvider {
    pub fn new() -> Self {
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

        Self {
            token_map: build_token_map(&legend),
            token_legend: legend,
        }
    }

    pub fn highlight_with_treesitter(&self, code: &str) -> Vec<SemanticToken> {
        let mut parser = tree_sitter::Parser::new();
        let language = tree_sitter_simfony::LANGUAGE;

        parser
            .set_language(&language.into())
            .expect("Error loading SimplicityHL parser");
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
        tokens
    }
}
