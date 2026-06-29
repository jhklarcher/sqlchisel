#[derive(Clone)]
pub(super) struct JinjaFragment {
    token: String,
    content: String,
    replacement: JinjaReplacement,
}

#[derive(Clone, Copy)]
enum JinjaReplacement {
    Inline,
    StandaloneLine,
    BlockComment,
}

pub(super) fn contains_jinja_markers(input: &str) -> bool {
    input.contains("{{") || input.contains("{%") || input.contains("{#")
}

pub(super) fn preserve_jinja_expressions(input: &str) -> (String, Vec<JinjaFragment>) {
    let mut out = String::with_capacity(input.len());
    let mut frags = Vec::new();
    let bytes = input.as_bytes();
    let mut idx = 0usize;

    while idx < bytes.len() {
        let c = bytes[idx] as char;

        if idx + 1 < bytes.len() && bytes[idx] == b'{' && bytes[idx + 1] == b'{' {
            let start = idx;
            let end = find_tag_end(input, idx + 2, b'}', b'}').unwrap_or(input.len());
            push_fragment(input, start, end, &mut out, &mut frags, JinjaTagKind::Expr);
            idx = end;
            continue;
        }

        if idx + 1 < bytes.len() && bytes[idx] == b'{' && bytes[idx + 1] == b'%' {
            let start = idx;
            let first_end = find_tag_end(input, idx + 2, b'%', b'}').unwrap_or(input.len());
            let end = if block_tag_name(&input[start..first_end]) == Some("raw") {
                find_endraw_block(input, first_end).unwrap_or(input.len())
            } else {
                first_end
            };
            push_fragment(input, start, end, &mut out, &mut frags, JinjaTagKind::Block);
            idx = end;
            continue;
        }

        if idx + 1 < bytes.len() && bytes[idx] == b'{' && bytes[idx + 1] == b'#' {
            let start = idx;
            let end = find_tag_end(input, idx + 2, b'#', b'}').unwrap_or(input.len());
            push_fragment(
                input,
                start,
                end,
                &mut out,
                &mut frags,
                JinjaTagKind::Comment,
            );
            idx = end;
            continue;
        }

        out.push(c);
        idx += 1;
    }

    (out, frags)
}

pub(super) fn restore_jinja_expressions(mut output: String, frags: Vec<JinjaFragment>) -> String {
    for frag in frags {
        match frag.replacement {
            JinjaReplacement::Inline => {
                output = output.replace(&frag.token, &frag.content);
            }
            JinjaReplacement::StandaloneLine => {
                let re = regex::Regex::new(&format!(
                    "(?m)^[ \\t]*--[ \\t]*{}[ \\t]*$",
                    regex::escape(&frag.token)
                ))
                .expect("regex");
                let replaced = re.replace_all(&output, frag.content.as_str()).into_owned();
                output = replaced.replace(&frag.token, &frag.content);
            }
            JinjaReplacement::BlockComment => {
                let re = regex::Regex::new(&format!(
                    r"/\*[ \t]*{}[ \t]*\*/",
                    regex::escape(&frag.token)
                ))
                .expect("regex");
                let replaced = re.replace_all(&output, frag.content.as_str()).into_owned();
                output = replaced.replace(&frag.token, &frag.content);
            }
        }
    }
    output
}

#[derive(Clone, Copy)]
enum JinjaTagKind {
    Expr,
    Block,
    Comment,
}

fn push_fragment(
    input: &str,
    start: usize,
    end: usize,
    out: &mut String,
    frags: &mut Vec<JinjaFragment>,
    kind: JinjaTagKind,
) {
    let content = &input[start..end];
    let token = match kind {
        JinjaTagKind::Expr => format!("SQLCHISEL_JINJA_EXPR_{}__", frags.len()),
        JinjaTagKind::Block => format!("SQLCHISEL_JINJA_BLOCK_{}__", frags.len()),
        JinjaTagKind::Comment => format!("SQLCHISEL_JINJA_COMMENT_{}__", frags.len()),
    };

    let replacement = if is_standalone_line(input, start, end) {
        out.push_str("-- ");
        out.push_str(&token);
        JinjaReplacement::StandaloneLine
    } else if matches!(kind, JinjaTagKind::Comment) {
        out.push_str("/* ");
        out.push_str(&token);
        out.push_str(" */");
        JinjaReplacement::BlockComment
    } else {
        out.push_str(&token);
        JinjaReplacement::Inline
    };

    frags.push(JinjaFragment {
        token,
        content: content.to_string(),
        replacement,
    });
}

fn is_standalone_line(input: &str, start: usize, end: usize) -> bool {
    let before = input[..start]
        .rsplit_once('\n')
        .map(|(_, tail)| tail)
        .unwrap_or(&input[..start]);
    let after = input[end..]
        .split_once('\n')
        .map(|(head, _)| head)
        .unwrap_or(&input[end..]);
    before.trim().is_empty() && after.trim().is_empty()
}

fn find_tag_end(input: &str, mut idx: usize, close_a: u8, close_b: u8) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    while idx + 1 < bytes.len() {
        let b = bytes[idx];

        if escaped {
            escaped = false;
            idx += 1;
            continue;
        }

        if in_single {
            if b == b'\\' {
                escaped = true;
            } else if b == b'\'' {
                in_single = false;
            }
            idx += 1;
            continue;
        }

        if in_double {
            if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_double = false;
            }
            idx += 1;
            continue;
        }

        if b == b'\'' {
            in_single = true;
            idx += 1;
            continue;
        }
        if b == b'"' {
            in_double = true;
            idx += 1;
            continue;
        }
        if b == close_a && bytes[idx + 1] == close_b {
            return Some(idx + 2);
        }

        idx += 1;
    }

    None
}

fn find_endraw_block(input: &str, mut idx: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    while idx + 1 < bytes.len() {
        if bytes[idx] == b'{' && bytes[idx + 1] == b'%' {
            let end = find_tag_end(input, idx + 2, b'%', b'}')?;
            if block_tag_name(&input[idx..end]) == Some("endraw") {
                return Some(end);
            }
            idx = end;
        } else {
            idx += 1;
        }
    }
    None
}

fn block_tag_name(raw: &str) -> Option<&str> {
    let inner = raw.strip_prefix("{%")?.strip_suffix("%}")?;
    let inner = inner
        .trim()
        .strip_prefix('-')
        .unwrap_or(inner.trim())
        .trim_start();
    let inner = inner.strip_suffix('-').unwrap_or(inner).trim_end();
    inner.split_whitespace().next()
}
