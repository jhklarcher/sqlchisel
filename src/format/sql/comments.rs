use anyhow::Result;
use sqlparser::tokenizer::{Token, Tokenizer, Whitespace};

use crate::config::DialectKind;

#[derive(Clone)]
pub(super) struct CommentFragment {
    index: usize,
    inline_with_prev: bool,
    leading: String,
    content: String,
}

pub(super) fn extract_comments(input: &str, dialect: &DialectKind) -> Result<Vec<CommentFragment>> {
    let dialect = super::dialect_for_kind(dialect);
    let tokens = Tokenizer::new(dialect.as_ref(), input).tokenize()?;

    let mut comments = Vec::new();
    let mut non_ws_idx = 0usize;
    let mut whitespace_since_non_ws = String::new();
    let mut saw_newline_since_non_ws = false;

    for tok in tokens {
        match tok {
            Token::Whitespace(ws) => match ws {
                Whitespace::SingleLineComment { .. } | Whitespace::MultiLineComment(_) => {
                    let text = ws.to_string();
                    let inline_with_prev = non_ws_idx > 0 && !saw_newline_since_non_ws;
                    let leading = whitespace_since_non_ws
                        .rsplit_once('\n')
                        .map(|(_, tail)| tail.to_string())
                        .unwrap_or_else(|| whitespace_since_non_ws.clone());
                    comments.push(CommentFragment {
                        index: non_ws_idx,
                        inline_with_prev,
                        leading,
                        content: text.clone(),
                    });
                    whitespace_since_non_ws.clear();
                    if let Some(pos) = text.rfind('\n') {
                        whitespace_since_non_ws.push_str(&text[pos + 1..]);
                        saw_newline_since_non_ws = true;
                    } else {
                        saw_newline_since_non_ws = false;
                    }
                }
                _ => {
                    let text = ws.to_string();
                    if text.contains('\n') {
                        whitespace_since_non_ws = text
                            .rsplit_once('\n')
                            .map(|(_, tail)| tail.to_string())
                            .unwrap_or_default();
                        saw_newline_since_non_ws = true;
                    } else {
                        whitespace_since_non_ws.push_str(&text);
                    }
                }
            },
            _ => {
                non_ws_idx += 1;
                whitespace_since_non_ws.clear();
                saw_newline_since_non_ws = false;
            }
        }
    }

    Ok(comments)
}

pub(super) fn reattach_comments(
    formatted: String,
    mut comments: Vec<CommentFragment>,
    dialect: &DialectKind,
) -> Result<String> {
    if comments.is_empty() {
        return Ok(formatted);
    }

    let dialect = super::dialect_for_kind(dialect);
    let tokens = Tokenizer::new(dialect.as_ref(), &formatted).tokenize()?;

    let mut out = String::with_capacity(formatted.len());
    let mut pending_ws = String::new();
    let mut non_ws_idx = 0usize;
    let mut comment_iter = comments.drain(..).peekable();
    let mut skip_leading_newline = false;
    let mut skip_leading_inline_ws = false;

    for token in tokens {
        match token {
            Token::Whitespace(ws) => pending_ws.push_str(&ws.to_string()),
            other => {
                let mut leading_ws = std::mem::take(&mut pending_ws);
                if skip_leading_newline {
                    trim_one_newline(&mut leading_ws);
                    skip_leading_newline = false;
                }
                if skip_leading_inline_ws {
                    if leading_ws.contains('\n') {
                        trim_trailing_inline_ws(&mut out);
                    } else {
                        leading_ws.clear();
                    }
                    skip_leading_inline_ws = false;
                }
                let (prefix, indent) = split_leading_ws(&leading_ws);
                out.push_str(&prefix);

                while let Some(c) = comment_iter.peek() {
                    if !c.inline_with_prev && c.index == non_ws_idx {
                        if !indent.is_empty() {
                            out.push_str(&indent);
                        }
                        out.push_str(&c.content);
                        if !ends_with_whitespace(&c.content) {
                            out.push('\n');
                        }
                        comment_iter.next();
                    } else {
                        break;
                    }
                }

                out.push_str(&indent);
                out.push_str(&other.to_string());
                non_ws_idx += 1;

                while let Some(c) = comment_iter.peek() {
                    if c.inline_with_prev && c.index == non_ws_idx {
                        let spacer = if c.leading.is_empty() {
                            " "
                        } else {
                            c.leading.as_str()
                        };
                        out.push_str(spacer);
                        out.push_str(&c.content);
                        if c.content.ends_with('\n') {
                            skip_leading_newline = true;
                        } else if !ends_with_whitespace(&c.content) {
                            out.push(' ');
                            skip_leading_inline_ws = true;
                        }
                        comment_iter.next();
                    } else {
                        break;
                    }
                }
            }
        }
    }

    let mut trailing_ws = pending_ws;
    if skip_leading_newline {
        trim_one_newline(&mut trailing_ws);
    }
    if skip_leading_inline_ws {
        if trailing_ws.contains('\n') {
            trim_trailing_inline_ws(&mut out);
        } else {
            trailing_ws.clear();
        }
    }
    let (prefix, indent) = split_leading_ws(&trailing_ws);
    out.push_str(&prefix);

    while let Some(c) = comment_iter.peek() {
        if c.index == non_ws_idx {
            if c.inline_with_prev {
                let spacer = if c.leading.is_empty() {
                    " "
                } else {
                    c.leading.as_str()
                };
                out.push_str(spacer);
                out.push_str(&c.content);
            } else {
                if !indent.is_empty() {
                    out.push_str(&indent);
                }
                out.push_str(&c.content);
                if !ends_with_whitespace(&c.content) {
                    out.push('\n');
                }
            }
            comment_iter.next();
        } else {
            break;
        }
    }

    out.push_str(&indent);
    Ok(out)
}

fn split_leading_ws(ws: &str) -> (String, String) {
    if let Some(pos) = ws.rfind('\n') {
        let (prefix, indent) = ws.split_at(pos + 1);
        (prefix.to_string(), indent.to_string())
    } else {
        (String::new(), ws.to_string())
    }
}

fn trim_one_newline(s: &mut String) {
    if let Some(rest) = s.strip_prefix("\r\n") {
        *s = rest.to_string();
    } else if let Some(rest) = s.strip_prefix('\n') {
        *s = rest.to_string();
    }
}

fn ends_with_whitespace(s: &str) -> bool {
    s.chars().next_back().is_some_and(|c| c.is_whitespace())
}

fn trim_trailing_inline_ws(out: &mut String) {
    while out.ends_with(' ') || out.ends_with('\t') {
        out.pop();
    }
}
