// src/query/highlight.rs

#![allow(dead_code)]

use std::ops::Range;

use gpui::{rgba, Rgba};
use sqlparser::dialect::GenericDialect;
use sqlparser::keywords::Keyword;
use sqlparser::tokenizer::{Token, Tokenizer, Whitespace};

pub type HighlightSpan = (Range<usize>, Rgba);

pub fn highlight(sql: &str) -> Vec<HighlightSpan> {
    if sql.is_empty() {
        return vec![];
    }
    let dialect = GenericDialect {};
    let tokens = match Tokenizer::new(&dialect, sql).tokenize_with_location() {
        Ok(t) => t,
        Err(_) => return vec![],
    };

    let line_starts = build_line_starts(sql);

    // Compute the byte-start of every token from its (line, col) location.
    let starts: Vec<usize> = tokens
        .iter()
        .map(|twl| {
            location_to_byte(
                sql,
                twl.span.start.line as usize,
                twl.span.start.column as usize,
                &line_starts,
            )
        })
        .collect();

    let mut spans = Vec::new();
    for (i, twl) in tokens.iter().enumerate() {
        let start = starts[i];
        // End of this token = start of the next token (stream is contiguous).
        // EOF token terminates at sql.len().
        let end = starts.get(i + 1).copied().unwrap_or(sql.len());
        if start >= end {
            continue;
        }
        if let Some(color) = token_color(&twl.token) {
            spans.push((start..end, color));
        }
    }
    spans
}

fn token_color(token: &Token) -> Option<Rgba> {
    match token {
        Token::Word(word) if word.keyword != Keyword::NoKeyword => {
            Some(rgba(0x89b4faff)) // blue — SQL keywords
        }
        Token::SingleQuotedString(_)
        | Token::DoubleQuotedString(_)
        | Token::EscapedStringLiteral(_)
        | Token::NationalStringLiteral(_)
        | Token::SingleQuotedByteStringLiteral(_)
        | Token::DoubleQuotedByteStringLiteral(_)
        | Token::HexStringLiteral(_) => Some(rgba(0xa6e3a1ff)), // green — strings
        Token::Number(_, _) => Some(rgba(0xfab387ff)),          // peach — numbers
        Token::Whitespace(Whitespace::SingleLineComment { .. })
        | Token::Whitespace(Whitespace::MultiLineComment(_)) => Some(rgba(0x6c7086ff)), // muted — comments
        Token::Eq
        | Token::Neq
        | Token::Lt
        | Token::Gt
        | Token::LtEq
        | Token::GtEq
        | Token::Plus
        | Token::Minus
        | Token::Mul
        | Token::Div
        | Token::Mod => Some(rgba(0x89dcebff)), // sky — operators
        _ => None, // identifiers, punctuation, whitespace → default (no span)
    }
}

/// Byte offset of the first character on each line (0-indexed).
fn build_line_starts(sql: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(sql.char_indices().filter_map(|(i, c)| {
            if c == '\n' {
                Some(i + 1)
            } else {
                None
            }
        }))
        .collect()
}

/// Convert a sqlparser Location (1-indexed line, 1-indexed character column)
/// to a byte offset in `sql`.
fn location_to_byte(sql: &str, line: usize, col: usize, line_starts: &[usize]) -> usize {
    let line_start_byte = line_starts
        .get(line.saturating_sub(1))
        .copied()
        .unwrap_or(sql.len());
    let line_text = &sql[line_start_byte..];
    line_text
        .char_indices()
        .nth(col.saturating_sub(1))
        .map(|(b, _)| line_start_byte + b)
        .unwrap_or(line_start_byte + line_text.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_returns_no_spans() {
        assert!(highlight("").is_empty());
    }

    #[test]
    fn test_keyword_is_blue() {
        let spans = highlight("SELECT");
        assert_eq!(spans.len(), 1);
        assert_eq!(&"SELECT"[spans[0].0.clone()], "SELECT");
        assert_eq!(spans[0].1, rgba(0x89b4faff));
    }

    #[test]
    fn test_string_is_green() {
        let spans = highlight("'hello'");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].1, rgba(0xa6e3a1ff));
    }

    #[test]
    fn test_number_is_peach() {
        let spans = highlight("42");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].1, rgba(0xfab387ff));
    }

    #[test]
    fn test_identifier_has_no_span() {
        let spans = highlight("users");
        assert!(spans.is_empty());
    }

    #[test]
    fn test_single_line_comment_is_muted() {
        let spans = highlight("-- a comment");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].1, rgba(0x6c7086ff));
    }

    #[test]
    fn test_block_comment_is_muted() {
        let spans = highlight("/* block */");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].1, rgba(0x6c7086ff));
    }

    #[test]
    fn test_eq_operator_is_sky() {
        let spans = highlight("=");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].1, rgba(0x89dcebff));
    }

    #[test]
    fn test_multiline_keywords() {
        let sql = "SELECT id\nFROM users";
        let spans = highlight(sql);
        let has_select = spans
            .iter()
            .any(|(r, c)| &sql[r.clone()] == "SELECT" && *c == rgba(0x89b4faff));
        let has_from = spans
            .iter()
            .any(|(r, c)| &sql[r.clone()] == "FROM" && *c == rgba(0x89b4faff));
        assert!(has_select, "SELECT not found in spans");
        assert!(has_from, "FROM not found in spans");
    }

    #[test]
    fn test_spans_within_bounds() {
        let sql = "SELECT id FROM users WHERE active = 1";
        let spans = highlight(sql);
        for (range, _) in &spans {
            assert!(
                range.end <= sql.len(),
                "span {range:?} out of bounds (sql len {})",
                sql.len()
            );
            assert!(range.start <= range.end, "inverted span {range:?}");
        }
    }

    #[test]
    fn test_malformed_sql_does_not_panic() {
        let _ = highlight("SELECT !!! @@@ ###");
    }
}
