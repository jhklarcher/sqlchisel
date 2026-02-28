use crate::config::FormatterConfig;
use crate::format::doc::Doc;
use crate::parser::{
    AlterTableReflectionCommand, DremioCommand, DremioVersionClause, VersionSelector,
};

use super::{
    apply_keyword_case, format_option_block, format_parenthesized_clause, keyword_doc,
    split_top_level_commas,
};

pub(super) fn format_dremio_version_clause(v: &DremioVersionClause, cfg: &FormatterConfig) -> Doc {
    let mut parts = Vec::new();

    if let Some(at) = &v.at {
        let text = match at {
            VersionSelector::Branch(name) => format!(
                "{} {} {name}",
                apply_keyword_case("AT", cfg),
                apply_keyword_case("BRANCH", cfg)
            ),
            VersionSelector::Tag(name) => format!(
                "{} {} {name}",
                apply_keyword_case("AT", cfg),
                apply_keyword_case("TAG", cfg)
            ),
            VersionSelector::Ref(name) => format!(
                "{} {} {name}",
                apply_keyword_case("AT", cfg),
                apply_keyword_case("REF", cfg)
            ),
            VersionSelector::Commit(name) => format!(
                "{} {} {name}",
                apply_keyword_case("AT", cfg),
                apply_keyword_case("COMMIT", cfg)
            ),
        };
        parts.push(Doc::Text(text));
    }

    if let Some(ts) = &v.as_of_timestamp {
        if !parts.is_empty() {
            parts.push(Doc::Line);
        }
        parts.push(Doc::Text(format!(
            "{} {} {} {ts}",
            apply_keyword_case("AS", cfg),
            apply_keyword_case("OF", cfg),
            apply_keyword_case("TIMESTAMP", cfg)
        )));
    }

    Doc::Group(parts)
}

pub(super) fn format_dremio_command(cmd: &DremioCommand, cfg: &FormatterConfig) -> Doc {
    use DremioCommand::*;
    match cmd {
        Use { path } => Doc::Text(format!("{} {path}", apply_keyword_case("USE", cfg))),
        AlterTableReflection(cmd) => format_alter_table_reflection(cmd, cfg),
        BranchTag { verb, kind, rest } => {
            let prefer_break = verb.eq_ignore_ascii_case("MERGE");
            if verb.eq_ignore_ascii_case("MERGE") && kind.eq_ignore_ascii_case("BRANCH") {
                format_merge_branch(rest, cfg)
            } else {
                format_command_with_rest(&[verb, kind], rest, cfg, prefer_break)
            }
        }
        Show { kind, rest } => format_command_with_rest(&["SHOW", kind], rest, cfg, false),
        VacuumCatalog { rest } => {
            format_command_with_rest(&["VACUUM", "CATALOG"], rest, cfg, false)
        }
        Pipe { verb, rest } => format_pipe(verb, rest, cfg),
        TableMaintenance { verb, rest } => {
            if verb.eq_ignore_ascii_case("COPY INTO TABLE") {
                format_copy_into(rest, cfg)
            } else {
                format_optimize_or_vacuum(rest, verb, cfg)
            }
        }
        CreateFolder {
            if_not_exists,
            path,
        } => {
            let mut parts = vec![
                apply_keyword_case("CREATE", cfg),
                apply_keyword_case("FOLDER", cfg),
            ];
            if *if_not_exists {
                parts.push(apply_keyword_case("IF", cfg));
                parts.push(apply_keyword_case("NOT", cfg));
                parts.push(apply_keyword_case("EXISTS", cfg));
            }
            if !path.is_empty() {
                parts.push(path.to_string());
            }
            Doc::Text(parts.join(" "))
        }
        AlterPds { rest } => format_alter_pds(rest, cfg),
        AnalyzeTable { rest } => format_command_with_rest(&["ANALYZE", "TABLE"], rest, cfg, true),
        ShowTableProperties { rest } => {
            format_command_with_rest(&["SHOW", "TABLE", "PROPERTIES"], rest, cfg, false)
        }
        AccelerationManage { verb, rest } => format_acceleration_manage(verb, rest, cfg),
        Acceleration { rest } => format_acceleration(rest, cfg),
        Reflection { verb, rest } => format_reflection(verb, rest, cfg),
        RolesUsers { kind, rest } => format_command_with_rest(&[kind], rest, cfg, false),
        RowColumnPolicies { rest } => {
            format_command_with_rest(&["ROW", "COLUMN", "POLICIES"], rest, cfg, false)
        }
        QueueTag { verb, kind, rest } => format_command_with_rest(&[verb, kind], rest, cfg, false),
    }
}

fn format_command_with_rest(
    parts: &[&str],
    rest: &str,
    cfg: &FormatterConfig,
    prefer_break: bool,
) -> Doc {
    let mut head = parts
        .iter()
        .map(|s| apply_keyword_case(s, cfg))
        .collect::<Vec<_>>()
        .join(" ");
    let rest = rest.trim();

    let inline_len = head.len() + if rest.is_empty() { 0 } else { 1 + rest.len() };

    let force_break = prefer_break && inline_len > cfg.line_length / 2;

    if rest.is_empty() || (!force_break && inline_len <= cfg.line_length) {
        if !rest.is_empty() {
            head.push(' ');
            head.push_str(rest);
        }
        Doc::Text(head)
    } else {
        Doc::Group(vec![
            Doc::Text(head),
            Doc::Line,
            Doc::Indent(Box::new(Doc::Text(rest.to_string()))),
        ])
    }
}

fn format_alter_pds(rest: &str, cfg: &FormatterConfig) -> Doc {
    let lower = rest.to_lowercase();
    for suffix in ["refresh metadata force update", "refresh metadata"] {
        if lower.ends_with(suffix) {
            let path_part = rest[..rest.len() - suffix.len()].trim_end();
            let suffix_doc = suffix
                .split_whitespace()
                .map(|kw| apply_keyword_case(kw, cfg))
                .collect::<Vec<_>>()
                .join(" ");
            let mut text = format!(
                "{} {}",
                apply_keyword_case("ALTER", cfg),
                apply_keyword_case("PDS", cfg)
            );
            if !path_part.is_empty() {
                text.push(' ');
                text.push_str(path_part);
            }
            if !suffix_doc.is_empty() {
                text.push(' ');
                text.push_str(&suffix_doc);
            }
            return Doc::Text(text);
        }
    }

    format_command_with_rest(&["ALTER", "PDS"], rest, cfg, false)
}

fn format_copy_into(rest: &str, cfg: &FormatterConfig) -> Doc {
    // Copy Into can have USING/WITH options; break into target / using / with blocks.
    let rest = rest.trim();
    if rest.is_empty() {
        return Doc::Text(apply_keyword_case("COPY INTO TABLE", cfg));
    }

    let (target, using_block, with_block) = split_copy_into_parts(rest);
    let mut parts = vec![
        Doc::Text(apply_keyword_case("COPY INTO TABLE", cfg)),
        Doc::Line,
    ];
    parts.push(Doc::Indent(Box::new(Doc::Text(target.to_string()))));

    if let Some(using) = using_block {
        parts.push(Doc::Line);
        parts.push(Doc::Indent(Box::new(format_option_block(
            "USING", &using, cfg,
        ))));
    }

    if let Some(with) = with_block {
        parts.push(Doc::Line);
        parts.push(Doc::Indent(Box::new(format_option_block(
            "WITH", &with, cfg,
        ))));
    }

    Doc::Group(parts)
}

fn format_pipe(verb: &str, rest: &str, cfg: &FormatterConfig) -> Doc {
    let rest = rest.trim();
    let head = format!(
        "{} {}",
        apply_keyword_case(verb, cfg),
        apply_keyword_case("PIPE", cfg)
    );
    if rest.is_empty() {
        return Doc::Text(head);
    }

    let mut parts = vec![Doc::Text(head.clone())];

    if let Some((name, opts)) = rest.split_once(' ') {
        let name = name.trim();
        parts.push(Doc::Space);
        parts.push(Doc::Text(name.to_string()));
        let opts = opts.trim();
        if !opts.is_empty() {
            let lower = opts.to_lowercase();
            if lower.starts_with("with") {
                parts.push(Doc::Line);
                parts.push(Doc::Indent(Box::new(format_option_block(
                    "WITH", opts, cfg,
                ))));
            } else if lower.starts_with("using") {
                parts.push(Doc::Line);
                parts.push(Doc::Indent(Box::new(format_option_block(
                    "USING", opts, cfg,
                ))));
            } else {
                let inline_len = head.len() + 1 + name.len() + 1 + opts.len();
                if inline_len > cfg.line_length {
                    parts.push(Doc::Line);
                    parts.push(Doc::Indent(Box::new(Doc::Text(opts.to_string()))));
                } else {
                    parts.push(Doc::Space);
                    parts.push(Doc::Text(opts.to_string()));
                }
            }
        }
    } else {
        parts.push(Doc::Space);
        parts.push(Doc::Text(rest.to_string()));
    }

    Doc::Group(parts)
}

fn format_optimize_or_vacuum(rest: &str, verb: &str, cfg: &FormatterConfig) -> Doc {
    // Format OPTIMIZE/VACUUM/ROLLBACK TABLE with optional WITH/USING options
    let rest = rest.trim();
    if rest.is_empty() {
        return Doc::Text(format!(
            "{} {}",
            apply_keyword_case(verb, cfg),
            apply_keyword_case("TABLE", cfg)
        ));
    }

    let mut parts = vec![Doc::Text(format!(
        "{} {}",
        apply_keyword_case(verb, cfg),
        apply_keyword_case("TABLE", cfg)
    ))];
    if let Some((table, opts)) = rest.split_once(' ') {
        let table = table.trim();
        parts.push(Doc::Space);
        parts.push(Doc::Text(table.to_string()));

        let opts = opts.trim();
        if !opts.is_empty() {
            let lower = opts.to_lowercase();
            if lower.starts_with("with") {
                parts.push(Doc::Line);
                parts.push(Doc::Indent(Box::new(format_option_block(
                    "WITH", opts, cfg,
                ))));
            } else if lower.starts_with("using") {
                parts.push(Doc::Line);
                parts.push(Doc::Indent(Box::new(format_option_block(
                    "USING", opts, cfg,
                ))));
            } else {
                let inline_len = verb.len() + " TABLE ".len() + table.len() + opts.len();
                if inline_len > cfg.line_length {
                    parts.push(Doc::Line);
                    parts.push(Doc::Indent(Box::new(Doc::Text(opts.to_string()))));
                } else {
                    parts.push(Doc::Space);
                    parts.push(Doc::Text(opts.to_string()));
                }
            }
        }
    } else {
        parts.push(Doc::Space);
        parts.push(Doc::Text(rest.to_string()));
    }

    Doc::Group(parts)
}

fn format_merge_branch(rest: &str, cfg: &FormatterConfig) -> Doc {
    let rest = rest.trim();
    if rest.is_empty() {
        return Doc::Text(apply_keyword_case("MERGE BRANCH", cfg));
    }

    let mut tokens: Vec<String> = rest.split_whitespace().map(|t| t.to_string()).collect();

    if tokens.is_empty() {
        return Doc::Text(apply_keyword_case("MERGE BRANCH", cfg));
    }

    let source = tokens.remove(0);
    let mut clauses: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for t in tokens {
        if matches_keyword(&t, &["into", "at", "as"]) {
            if !current.is_empty() {
                clauses.push(std::mem::take(&mut current));
            }
            current.push(t);
        } else {
            current.push(t);
        }
    }
    if !current.is_empty() {
        clauses.push(current);
    }

    let mut parts = vec![
        Doc::Text(apply_keyword_case("MERGE BRANCH", cfg)),
        Doc::Line,
        Doc::Indent(Box::new(Doc::Text(source))),
    ];

    for clause in clauses {
        if clause.is_empty() {
            continue;
        }
        let mut text = apply_keyword_case(&clause[0], cfg);
        if clause.len() > 1 {
            text.push(' ');
            text.push_str(&clause[1..].join(" "));
        }
        parts.push(Doc::Line);
        parts.push(Doc::Indent(Box::new(Doc::Text(text))));
    }

    Doc::Group(parts)
}

fn matches_keyword(tok: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|k| tok.eq_ignore_ascii_case(k))
}

fn format_acceleration(rest: &str, cfg: &FormatterConfig) -> Doc {
    let rest = rest.trim();
    if rest.is_empty() {
        return Doc::Text(apply_keyword_case("ACCELERATION", cfg));
    }

    if let Some(pos) = find_keyword_case_insensitive(rest, "with") {
        let target = rest[..pos].trim();
        let with_part = rest[pos..].trim();
        return Doc::Group(vec![
            Doc::Text(apply_keyword_case("ACCELERATION", cfg)),
            Doc::Line,
            Doc::Indent(Box::new(Doc::Text(target.to_string()))),
            Doc::Line,
            Doc::Indent(Box::new(format_option_block("WITH", with_part, cfg))),
        ]);
    }

    if let Some(pos) = find_keyword_case_insensitive(rest, "using") {
        let target = rest[..pos].trim();
        let using_part = rest[pos..].trim();
        return Doc::Group(vec![
            Doc::Text(apply_keyword_case("ACCELERATION", cfg)),
            Doc::Line,
            Doc::Indent(Box::new(Doc::Text(target.to_string()))),
            Doc::Line,
            Doc::Indent(Box::new(format_option_block("USING", using_part, cfg))),
        ]);
    }

    let inline_len = "ACCELERATION".len() + 1 + rest.len();
    if inline_len <= cfg.line_length {
        Doc::Text(format!(
            "{} {rest}",
            apply_keyword_case("ACCELERATION", cfg)
        ))
    } else {
        Doc::Group(vec![
            Doc::Text(apply_keyword_case("ACCELERATION", cfg)),
            Doc::Line,
            Doc::Indent(Box::new(Doc::Text(rest.to_string()))),
        ])
    }
}

fn format_acceleration_manage(verb: &str, rest: &str, cfg: &FormatterConfig) -> Doc {
    let head = format!(
        "{} {}",
        apply_keyword_case(verb, cfg),
        apply_keyword_case("ACCELERATION", cfg)
    );
    let rest = rest.trim();
    if rest.is_empty() {
        return Doc::Text(head);
    }

    if let Some((kw, pos)) = find_keyword_case_insensitive(rest, "with")
        .map(|p| ("WITH", p))
        .or_else(|| find_keyword_case_insensitive(rest, "using").map(|p| ("USING", p)))
    {
        let target = rest[..pos].trim();
        let options = rest[pos..].trim();
        return Doc::Group(vec![
            Doc::Text(head),
            Doc::Line,
            Doc::Indent(Box::new(Doc::Text(target.to_string()))),
            Doc::Line,
            Doc::Indent(Box::new(format_option_block(kw, options, cfg))),
        ]);
    }

    let inline_len = head.len() + 1 + rest.len();
    if inline_len <= cfg.line_length {
        Doc::Text(format!("{head} {rest}"))
    } else {
        Doc::Group(vec![
            Doc::Text(head),
            Doc::Line,
            Doc::Indent(Box::new(Doc::Text(rest.to_string()))),
        ])
    }
}

fn format_alter_table_reflection(cmd: &AlterTableReflectionCommand, cfg: &FormatterConfig) -> Doc {
    let mut parts = vec![Doc::Text(format!(
        "{} {}",
        apply_keyword_case("ALTER TABLE", cfg),
        cmd.table
    ))];

    let mut create_bits = vec![apply_keyword_case("CREATE", cfg)];
    if let Some(kind) = &cmd.kind {
        create_bits.push(apply_keyword_case(kind, cfg));
    }
    create_bits.push(apply_keyword_case("REFLECTION", cfg));
    if let Some(name) = &cmd.name {
        create_bits.push(name.clone());
    }

    parts.push(Doc::Line);
    parts.push(Doc::Text(create_bits.join(" ")));

    for (label, value) in &cmd.clauses {
        let clause = format_reflection_clause(label, value, cfg);
        parts.push(Doc::Line);
        parts.push(clause);
    }

    Doc::Group(parts)
}

fn format_reflection_clause(label: &str, value: &str, cfg: &FormatterConfig) -> Doc {
    let trimmed = value.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner = &trimmed[1..trimmed.len() - 1];
        let items = split_top_level_commas(inner);
        return format_parenthesized_clause(label, items, cfg, true);
    }

    let mut parts = vec![keyword_doc(cfg, label)];
    if !trimmed.is_empty() {
        parts.push(Doc::Space);
        parts.push(Doc::Text(trimmed.to_string()));
    }
    Doc::Group(parts)
}

fn format_reflection(verb: &str, rest: &str, cfg: &FormatterConfig) -> Doc {
    let head = format!(
        "{} {}",
        apply_keyword_case(verb, cfg),
        apply_keyword_case("REFLECTION", cfg)
    );
    let rest = rest.trim();
    if rest.is_empty() {
        return Doc::Text(head);
    }

    if let Some((kw, pos)) = find_keyword_case_insensitive(rest, "with")
        .map(|p| ("WITH", p))
        .or_else(|| find_keyword_case_insensitive(rest, "using").map(|p| ("USING", p)))
    {
        let target = rest[..pos].trim();
        let options = rest[pos..].trim();
        return format_reflection_with_options(&head, target, kw, options, cfg);
    }

    let inline_len = head.len() + 1 + rest.len();
    if inline_len <= cfg.line_length {
        Doc::Text(format!("{head} {rest}"))
    } else {
        Doc::Group(vec![
            Doc::Text(head),
            Doc::Line,
            Doc::Indent(Box::new(Doc::Text(rest.to_string()))),
        ])
    }
}

fn format_reflection_with_options(
    head: &str,
    target: &str,
    keyword: &str,
    options: &str,
    cfg: &FormatterConfig,
) -> Doc {
    let mut parts = vec![Doc::Text(head.to_string())];

    if !target.is_empty() {
        parts.push(Doc::Line);
        parts.push(Doc::Indent(Box::new(Doc::Text(target.to_string()))));
    }

    let option_doc = format_reflection_options(keyword, options, cfg);
    parts.push(Doc::Line);
    parts.push(Doc::Indent(Box::new(option_doc)));

    Doc::Group(parts)
}

fn format_reflection_options(keyword: &str, options: &str, cfg: &FormatterConfig) -> Doc {
    let trimmed = options.trim();
    let lower_kw = keyword.to_lowercase();
    let rest = if trimmed.to_lowercase().starts_with(&lower_kw) {
        trimmed[lower_kw.len()..].trim()
    } else {
        trimmed
    };

    let clauses = split_reflection_clauses(rest);
    if clauses.is_empty() {
        return Doc::Group(vec![
            keyword_doc(cfg, keyword),
            Doc::Space,
            Doc::Text(rest.to_string()),
        ]);
    }

    let mut lines = Vec::new();
    for (idx, (label, value)) in clauses.iter().enumerate() {
        lines.push(Doc::Group(vec![
            keyword_doc(cfg, label),
            Doc::Space,
            Doc::Text(value.to_string()),
        ]));
        if idx + 1 < clauses.len() {
            lines.push(Doc::Line);
        }
    }

    Doc::Group(vec![
        keyword_doc(cfg, keyword),
        Doc::Line,
        Doc::Indent(Box::new(Doc::Group(lines))),
    ])
}

fn split_reflection_clauses(options: &str) -> Vec<(String, String)> {
    let lower = options.to_lowercase();
    let keywords = [
        ("TABLE", "table"),
        ("DISPLAY NAME", "display name"),
        ("PARTITION BY", "partition by"),
        ("DISTRIBUTE BY", "distribute by"),
        ("SORT BY", "sort by"),
    ];

    let mut positions: Vec<(usize, &(&str, &str))> = Vec::new();
    for kw in &keywords {
        if let Some(pos) = lower.find(kw.1) {
            positions.push((pos, kw));
        }
    }
    positions.sort_by_key(|(pos, _)| *pos);

    let mut clauses = Vec::new();
    let prefix_end = positions
        .first()
        .map(|(p, _)| *p)
        .unwrap_or_else(|| lower.len());
    let prefix = options[..prefix_end].trim();
    if !prefix.is_empty() {
        if prefix.to_lowercase().starts_with("table ") {
            let value = prefix[5..].trim();
            clauses.push(("TABLE".to_string(), value.to_string()));
        } else {
            let mut parts = prefix.splitn(2, char::is_whitespace);
            if let Some(label) = parts.next() {
                let value = parts.next().unwrap_or("").trim();
                clauses.push((label.to_uppercase(), value.to_string()));
            }
        }
    }

    for (idx, (pos, (label, raw_kw))) in positions.iter().enumerate() {
        let start = *pos;
        let end = positions
            .get(idx + 1)
            .map(|(p, _)| *p)
            .unwrap_or_else(|| lower.len());
        if end <= start {
            continue;
        }
        let segment = options[start..end].trim();
        let value = segment[raw_kw.len()..].trim();
        clauses.push((label.to_string(), value.to_string()));
    }

    clauses
}

fn split_copy_into_parts(rest: &str) -> (String, Option<String>, Option<String>) {
    let using_pos = find_keyword_case_insensitive(rest, "using");
    let with_pos = find_keyword_case_insensitive(rest, "with");

    let mut target = rest;
    let mut using_block = None;
    let mut with_block = None;

    // prefer earliest keyword split
    if let Some(pos) = using_pos.or(with_pos) {
        target = rest[..pos].trim();
        let remainder = rest[pos..].trim();

        if let Some(upos) = using_pos {
            if upos == pos {
                let (using_text, rem) = if let Some(wpos) = with_pos {
                    if wpos > upos {
                        remainder.split_at(wpos - pos)
                    } else {
                        (remainder, "")
                    }
                } else {
                    (remainder, "")
                };
                using_block = Some(using_text.trim().to_string());
                let rem = rem.trim();
                if !rem.is_empty() {
                    with_block = Some(rem.to_string());
                }
            } else if let Some(wpos) = with_pos {
                if wpos == pos {
                    with_block = Some(remainder.to_string());
                }
            }
        } else if with_pos.is_some() {
            with_block = Some(remainder.to_string());
        }
    }

    (target.to_string(), using_block, with_block)
}

fn find_keyword_case_insensitive(haystack: &str, keyword: &str) -> Option<usize> {
    let hl = haystack.to_lowercase();
    let kl = keyword.to_lowercase();
    let mut offset = 0;
    let bytes = hl.as_bytes();
    while let Some(pos) = hl[offset..].find(&kl) {
        let abs = offset + pos;
        let start_ok = abs == 0 || bytes[abs - 1].is_ascii_whitespace();
        let end_pos = abs + kl.len();
        let end_ok = end_pos >= bytes.len() || bytes[end_pos].is_ascii_whitespace();
        if start_ok && end_ok {
            return Some(abs);
        }
        offset = abs + 1;
    }
    None
}
