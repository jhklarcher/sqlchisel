use crate::format::doc::Doc;

#[derive(Debug, Clone)]
pub struct PrintConfig {
    pub line_length: usize,
    pub indent_width: usize,
}

impl Default for PrintConfig {
    fn default() -> Self {
        Self {
            line_length: 100,
            indent_width: 2,
        }
    }
}

#[derive(Copy, Clone)]
enum Mode {
    Break,
    Flat,
}

#[derive(Copy, Clone)]
struct Frame<'a> {
    indent: usize,
    mode: Mode,
    doc: &'a Doc,
}

pub fn format_doc(doc: &Doc, config: &PrintConfig) -> String {
    let mut out = String::new();
    let mut col = 0usize;
    let mut line_start = false;
    let mut stack = vec![Frame {
        indent: 0,
        mode: Mode::Break,
        doc,
    }];

    while let Some(Frame { indent, mode, doc }) = stack.pop() {
        match doc {
            Doc::Text(s) => {
                if line_start {
                    push_indent(&mut out, indent);
                    col += indent;
                    line_start = false;
                }
                out.push_str(s);
                col += s.len();
            }
            Doc::Space => {
                if line_start {
                    push_indent(&mut out, indent);
                    col += indent;
                    line_start = false;
                }
                out.push(' ');
                col += 1;
            }
            Doc::Line => {
                out.push('\n');
                line_start = true;
                col = 0;
            }
            Doc::SoftLine => match mode {
                Mode::Flat => {
                    if line_start {
                        push_indent(&mut out, indent);
                        col += indent;
                        line_start = false;
                    }
                    out.push(' ');
                    col += 1;
                }
                Mode::Break => {
                    out.push('\n');
                    line_start = true;
                    col = 0;
                }
            },
            Doc::Indent(inner) => {
                stack.push(Frame {
                    indent: indent + config.indent_width,
                    mode,
                    doc: inner,
                });
            }
            Doc::Group(children) => {
                let remaining = config.line_length.saturating_sub(col) as isize;
                let fits = fits(remaining, children, indent, config);
                let group_mode = if fits { Mode::Flat } else { Mode::Break };
                push_children(&mut stack, children, indent, group_mode);
            }
            Doc::Concat(children) => {
                push_children(&mut stack, children, indent, mode);
            }
        }
    }

    out
}

fn push_children<'a>(stack: &mut Vec<Frame<'a>>, children: &'a [Doc], indent: usize, mode: Mode) {
    for child in children.iter().rev() {
        stack.push(Frame {
            indent,
            mode,
            doc: child,
        });
    }
}

fn push_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push(' ');
    }
}

fn fits(mut remaining: isize, children: &[Doc], indent: usize, config: &PrintConfig) -> bool {
    let mut stack: Vec<Frame> = children
        .iter()
        .rev()
        .map(|doc| Frame {
            indent,
            mode: Mode::Flat,
            doc,
        })
        .collect();

    while let Some(Frame { indent, mode, doc }) = stack.pop() {
        if remaining < 0 {
            return false;
        }

        match doc {
            Doc::Text(s) => {
                remaining -= s.len() as isize;
            }
            Doc::Space => {
                remaining -= 1;
            }
            Doc::Line => match mode {
                Mode::Flat => remaining -= 1,
                Mode::Break => return true,
            },
            Doc::SoftLine => match mode {
                Mode::Flat => remaining -= 1,
                Mode::Break => return true,
            },
            Doc::Indent(inner) => stack.push(Frame {
                indent: indent + config.indent_width,
                mode,
                doc: inner,
            }),
            Doc::Group(children) => push_children(&mut stack, children, indent, Mode::Flat),
            Doc::Concat(children) => push_children(&mut stack, children, indent, mode),
        }
    }

    remaining >= 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::doc::Doc;

    #[test]
    fn group_fits_on_single_line() {
        let doc = Doc::Group(vec![
            Doc::Text("SELECT".into()),
            Doc::Space,
            Doc::Text("a".into()),
            Doc::SoftLine,
            Doc::Text("b".into()),
        ]);
        let cfg = PrintConfig {
            line_length: 40,
            indent_width: 2,
        };
        let rendered = format_doc(&doc, &cfg);
        assert_eq!(rendered, "SELECT a b");
    }

    #[test]
    fn group_breaks_when_too_long() {
        let doc = Doc::Group(vec![
            Doc::Text("SELECT".into()),
            Doc::Space,
            Doc::Text("a_long_identifier".into()),
            Doc::SoftLine,
            Doc::Text("b_long_identifier".into()),
        ]);
        let cfg = PrintConfig {
            line_length: 12,
            indent_width: 2,
        };
        let rendered = format_doc(&doc, &cfg);
        assert_eq!(rendered, "SELECT a_long_identifier\nb_long_identifier");
    }

    #[test]
    fn indent_applies_to_nested_lines() {
        let doc = Doc::Group(vec![
            Doc::Text("SELECT".into()),
            Doc::Line,
            Doc::Indent(Box::new(Doc::Concat(vec![
                Doc::Text("a".into()),
                Doc::Line,
                Doc::Text("b".into()),
            ]))),
        ]);
        let cfg = PrintConfig {
            line_length: 20,
            indent_width: 2,
        };
        let rendered = format_doc(&doc, &cfg);
        assert_eq!(rendered, "SELECT\n  a\n  b");
    }
}
