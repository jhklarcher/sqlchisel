pub(super) fn preserve_string_literals(input: &str) -> (String, Vec<String>) {
    let mut out = String::with_capacity(input.len());
    let mut literals = Vec::new();
    let bytes = input.as_bytes();
    let mut idx = 0usize;

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

        if c == '\'' {
            let start = idx;
            idx += 1;
            while idx < bytes.len() {
                if bytes[idx] == b'\'' {
                    if idx + 1 < bytes.len() && bytes[idx + 1] == b'\'' {
                        idx += 2;
                        continue;
                    }
                    idx += 1;
                    break;
                }
                idx += 1;
            }
            let raw = &input[start..idx];
            let placeholder = format!("'__SQLCHISEL_LITERAL_{}__'", literals.len());
            literals.push(raw.to_string());
            out.push_str(&placeholder);
        } else {
            out.push(c);
            idx += 1;
        }
    }
    (out, literals)
}

pub(super) fn restore_string_literals(mut output: String, literals: Vec<String>) -> String {
    for (idx, raw) in literals.iter().enumerate() {
        let placeholder = format!("'__SQLCHISEL_LITERAL_{}__'", idx);
        output = output.replace(&placeholder, raw);
    }
    output
}
