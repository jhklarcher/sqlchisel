#[derive(Clone)]
pub(super) struct JinjaFragment {
    placeholder: String,
    content: String,
    kind: JinjaKind,
}

#[derive(Clone, Copy)]
enum JinjaKind {
    Expr,
    Block,
}

pub(super) fn contains_jinja_markers(input: &str) -> bool {
    input.contains("{{") || input.contains("{%") || input.contains("{#")
}

pub(super) fn preserve_jinja_expressions(input: &str) -> (String, Vec<JinjaFragment>) {
    let mut out = String::with_capacity(input.len());
    let mut frags = Vec::new();
    let bytes = input.as_bytes();
    let mut idx = 0usize;

    let mut in_single = false;
    let mut in_double = false;
    let mut in_line_comment = false;
    let mut in_block_comment = 0usize;

    while idx < bytes.len() {
        let c = bytes[idx] as char;

        if in_line_comment {
            out.push(c);
            if c == '\n' {
                in_line_comment = false;
            }
            idx += 1;
            continue;
        }

        if in_block_comment > 0 {
            out.push(c);
            if c == '/' && idx + 1 < bytes.len() && bytes[idx + 1] == b'*' {
                in_block_comment += 1;
            } else if c == '*' && idx + 1 < bytes.len() && bytes[idx + 1] == b'/' {
                in_block_comment -= 1;
            }
            idx += 1;
            continue;
        }

        if in_single {
            out.push(c);
            if c == '\'' {
                in_single = false;
            }
            idx += 1;
            continue;
        }

        if in_double {
            out.push(c);
            if c == '"' {
                in_double = false;
            }
            idx += 1;
            continue;
        }

        if c == '-' && idx + 1 < bytes.len() && bytes[idx + 1] == b'-' {
            out.push_str("--");
            idx += 2;
            in_line_comment = true;
            continue;
        }
        if c == '/' && idx + 1 < bytes.len() && bytes[idx + 1] == b'*' {
            out.push_str("/*");
            idx += 2;
            in_block_comment = 1;
            continue;
        }

        // Detect inline jinja expression {{ ... }} even if it spans multiple characters
        if idx + 1 < bytes.len() && bytes[idx] == b'{' && bytes[idx + 1] == b'{' {
            let start = idx;
            idx += 2;
            while idx + 1 < bytes.len() {
                if bytes[idx] == b'}' && bytes[idx + 1] == b'}' {
                    idx += 2;
                    break;
                }
                idx += 1;
            }
            let raw = &input[start..idx];
            let placeholder = format!("JINJA_EXPR_{}", frags.len());
            frags.push(JinjaFragment {
                placeholder: placeholder.clone(),
                content: raw.to_string(),
                kind: JinjaKind::Expr,
            });
            out.push_str(&placeholder);
            continue;
        }

        // Detect jinja block/comment tags and keep them as placeholders so we can reinsert later.
        if idx + 1 < bytes.len()
            && bytes[idx] == b'{'
            && (bytes[idx + 1] == b'%' || bytes[idx + 1] == b'#')
        {
            let start = idx;
            idx += 2;
            while idx + 1 < bytes.len() {
                if (bytes[idx] == b'%' || bytes[idx] == b'#') && bytes[idx + 1] == b'}' {
                    idx += 2;
                    break;
                }
                idx += 1;
            }
            let raw = &input[start..idx];
            let placeholder = format!("JINJA_BLOCK_{}", frags.len());
            frags.push(JinjaFragment {
                placeholder: placeholder.clone(),
                content: raw.to_string(),
                kind: JinjaKind::Block,
            });
            out.push_str(&placeholder);
            continue;
        }

        if c == '\'' {
            in_single = true;
        } else if c == '"' {
            in_double = true;
        }

        out.push(c);
        idx += 1;
    }

    (out, frags)
}

pub(super) fn restore_jinja_expressions(mut output: String, frags: Vec<JinjaFragment>) -> String {
    for frag in frags {
        match frag.kind {
            JinjaKind::Expr => {
                output = output.replace(&frag.placeholder, &frag.content);
            }
            JinjaKind::Block => {
                // Replace with optional leading whitespace trimmed so block tags return to column 0.
                let re = regex::Regex::new(&format!(
                    "(?m)^\\s*{}\\s*$",
                    regex::escape(&frag.placeholder)
                ))
                .expect("regex");
                let replaced = re.replace_all(&output, frag.content.as_str()).into_owned();
                output = replaced.replace(&frag.placeholder, &frag.content);
            }
        }
    }
    output
}
