use crate::config::DialectKind;
use regex::Regex;
use sqlparser::ast::{Query, SetExpr, Statement, TableFactor, TableWithJoins, With};
use sqlparser::dialect::{AnsiDialect, Dialect, GenericDialect};
use sqlparser::parser::{Parser, ParserError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionSelector {
    Branch(String),
    Tag(String),
    Ref(String),
    Commit(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DremioVersionClause {
    pub at: Option<VersionSelector>,
    pub as_of_timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DremioCommand {
    Use {
        path: String,
    },
    AlterTableReflection(AlterTableReflectionCommand),
    AlterPds {
        rest: String,
    },
    BranchTag {
        verb: String,
        kind: String,
        rest: String,
    },
    Show {
        kind: String,
        rest: String,
    },
    VacuumCatalog {
        rest: String,
    },
    Pipe {
        verb: String,
        rest: String,
    },
    TableMaintenance {
        verb: String,
        rest: String,
    },
    AnalyzeTable {
        rest: String,
    },
    ShowTableProperties {
        rest: String,
    },
    Acceleration {
        rest: String,
    },
    AccelerationManage {
        verb: String,
        rest: String,
    },
    Reflection {
        verb: String,
        rest: String,
    },
    RolesUsers {
        kind: String,
        rest: String,
    },
    RowColumnPolicies {
        rest: String,
    },
    QueueTag {
        verb: String,
        kind: String,
        rest: String,
    },
    CreateFolder {
        if_not_exists: bool,
        path: String,
    },
    Generic {
        head: Vec<String>,
        rest: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlterTableReflectionCommand {
    pub table: String,
    pub kind: Option<String>,
    pub name: Option<String>,
    pub clauses: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedStatement {
    Sql {
        stmt: Box<Statement>,
        version: Option<DremioVersionClause>,
        has_semicolon: bool,
        relation_alias_has_as: Vec<bool>,
    },
    Command {
        cmd: DremioCommand,
        has_semicolon: bool,
    },
    Raw {
        sql: String,
        has_semicolon: bool,
    },
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ParseOptions {
    pub strict: bool,
}

#[allow(dead_code)]
pub fn parse_sql(input: &str, dialect: DialectKind) -> Result<Vec<ParsedStatement>, ParserError> {
    parse_sql_with_options(input, dialect, ParseOptions::default())
}

pub fn parse_sql_with_options(
    input: &str,
    dialect: DialectKind,
    options: ParseOptions,
) -> Result<Vec<ParsedStatement>, ParserError> {
    parse_sql_for_dialect(input, dialect, options)
}

fn parse_sql_for_dialect(
    input: &str,
    dialect: DialectKind,
    options: ParseOptions,
) -> Result<Vec<ParsedStatement>, ParserError> {
    let fragments = split_sql_fragments(input, dialect)?;
    match dialect {
        DialectKind::Ansi => parse_ansi(fragments, options),
        DialectKind::Dremio => parse_dremio(fragments, options),
    }
}

#[derive(Debug)]
struct StatementFragment {
    sql: String,
    has_semicolon: bool,
}

fn split_sql_fragments(
    input: &str,
    _dialect: DialectKind,
) -> Result<Vec<StatementFragment>, ParserError> {
    let mut fragments = Vec::new();
    let mut start = 0usize;
    let mut idx = 0usize;
    let bytes = input.as_bytes();

    let mut in_single = false;
    let mut in_double = false;
    let mut in_line_comment = false;
    let mut in_block_comment = 0usize;

    let mut push_fragment = |slice: &str, has_semicolon: bool| {
        let trimmed = slice.trim();
        if trimmed.is_empty() {
            return;
        }
        fragments.push(StatementFragment {
            sql: trimmed.to_string(),
            has_semicolon,
        });
    };

    while idx < bytes.len() {
        let c = bytes[idx] as char;

        if in_line_comment {
            if c == '\n' {
                in_line_comment = false;
            }
            idx += 1;
            continue;
        }

        if in_block_comment > 0 {
            if c == '/' && idx + 1 < bytes.len() && bytes[idx + 1] == b'*' {
                in_block_comment += 1;
                idx += 2;
                continue;
            }
            if c == '*' && idx + 1 < bytes.len() && bytes[idx + 1] == b'/' {
                in_block_comment -= 1;
                idx += 2;
                continue;
            }
            idx += 1;
            continue;
        }

        if in_single {
            if c == '\'' {
                if idx + 1 < bytes.len() && bytes[idx + 1] == b'\'' {
                    idx += 2;
                    continue;
                }
                in_single = false;
            }
            idx += 1;
            continue;
        }

        if in_double {
            if c == '"' {
                if idx + 1 < bytes.len() && bytes[idx + 1] == b'"' {
                    idx += 2;
                    continue;
                }
                in_double = false;
            }
            idx += 1;
            continue;
        }

        // outside quotes/comments
        if c == '-' && idx + 1 < bytes.len() && bytes[idx + 1] == b'-' {
            in_line_comment = true;
            idx += 2;
            continue;
        }
        if c == '/' && idx + 1 < bytes.len() && bytes[idx + 1] == b'*' {
            in_block_comment = 1;
            idx += 2;
            continue;
        }
        if c == '\'' {
            in_single = true;
            idx += 1;
            continue;
        }
        if c == '"' {
            in_double = true;
            idx += 1;
            continue;
        }
        if c == ';' {
            let fragment = &input[start..idx];
            push_fragment(fragment, true);
            idx += 1;
            start = idx;
            continue;
        }

        idx += 1;
    }

    if start <= input.len() {
        let fragment = &input[start..];
        push_fragment(fragment, false);
    }

    Ok(fragments)
}

fn parse_ansi(
    fragments: Vec<StatementFragment>,
    options: ParseOptions,
) -> Result<Vec<ParsedStatement>, ParserError> {
    let dialect = select_dialect(DialectKind::Ansi);
    let mut out = Vec::new();

    for fragment in fragments {
        let normalized = normalize_overescaped_literals(&fragment.sql);
        let mut stmts = match Parser::parse_sql(&*dialect, normalized) {
            Ok(stmts) => stmts,
            Err(err) => {
                if options.strict {
                    return Err(err);
                }
                out.push(ParsedStatement::Raw {
                    sql: fragment.sql,
                    has_semicolon: fragment.has_semicolon,
                });
                continue;
            }
        };
        for stmt in stmts.drain(..) {
            let relation_alias_has_as = collect_relation_alias_flags(&stmt);
            out.push(ParsedStatement::Sql {
                stmt: Box::new(stmt),
                version: None,
                has_semicolon: fragment.has_semicolon,
                relation_alias_has_as,
            });
        }
    }

    Ok(out)
}

fn parse_dremio(
    fragments: Vec<StatementFragment>,
    options: ParseOptions,
) -> Result<Vec<ParsedStatement>, ParserError> {
    let dialect = select_dialect(DialectKind::Dremio);
    let mut out = Vec::new();

    for fragment in fragments {
        let trimmed = fragment.sql.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(cmd) = parse_dremio_command(trimmed) {
            out.push(ParsedStatement::Command {
                cmd,
                has_semicolon: fragment.has_semicolon,
            });
            continue;
        }

        let parsed = strip_dremio_version_clause(trimmed);
        let normalized = normalize_overescaped_literals(&parsed.sql);
        let mut stmts = match Parser::parse_sql(&*dialect, normalized) {
            Ok(stmts) => stmts,
            Err(err) => {
                if options.strict {
                    return Err(err);
                }
                out.push(ParsedStatement::Raw {
                    sql: fragment.sql,
                    has_semicolon: fragment.has_semicolon,
                });
                continue;
            }
        };
        if let Some(stmt) = stmts.pop() {
            let relation_alias_has_as = collect_relation_alias_flags(&stmt);
            out.push(ParsedStatement::Sql {
                stmt: Box::new(stmt),
                version: parsed.version,
                has_semicolon: fragment.has_semicolon,
                relation_alias_has_as,
            });
        }
    }
    Ok(out)
}

fn collect_relation_alias_flags(stmt: &Statement) -> Vec<bool> {
    let mut flags = Vec::new();
    collect_aliases_from_statement(stmt, &mut flags);
    flags
}

fn collect_aliases_from_statement(stmt: &Statement, flags: &mut Vec<bool>) {
    match stmt {
        Statement::Query(query) => collect_aliases_from_query(query.as_ref(), flags),
        Statement::CreateTable(create_table) => {
            if let Some(q) = &create_table.query {
                collect_aliases_from_query(q, flags);
            }
        }
        Statement::CreateView(create_view) => {
            collect_aliases_from_query(create_view.query.as_ref(), flags);
        }
        Statement::Insert(insert) => {
            if let Some(query) = &insert.source {
                collect_aliases_from_query(query, flags);
            }
        }
        _ => {}
    };
}

fn collect_aliases_from_query(query: &Query, flags: &mut Vec<bool>) {
    if let Some(with) = &query.with {
        collect_aliases_from_with(with, flags);
    }

    match query.body.as_ref() {
        SetExpr::Select(select) => collect_aliases_from_select(select, flags),
        SetExpr::Query(inner) => collect_aliases_from_query(inner.as_ref(), flags),
        _ => {}
    }
}

fn collect_aliases_from_with(with: &With, flags: &mut Vec<bool>) {
    for cte in &with.cte_tables {
        collect_aliases_from_query(cte.query.as_ref(), flags);
    }
}

fn collect_aliases_from_select(select: &sqlparser::ast::Select, flags: &mut Vec<bool>) {
    for rel in &select.from {
        collect_aliases_from_table_factor(&rel.relation, flags);
        for join in &rel.joins {
            collect_aliases_from_table_factor(&join.relation, flags);
        }
    }
}

fn collect_aliases_from_table_with_joins(rel: &TableWithJoins, flags: &mut Vec<bool>) {
    collect_aliases_from_table_factor(&rel.relation, flags);
    for join in &rel.joins {
        collect_aliases_from_table_factor(&join.relation, flags);
    }
}

fn collect_aliases_from_table_factor(factor: &TableFactor, flags: &mut Vec<bool>) {
    match factor {
        TableFactor::Derived {
            subquery, alias, ..
        } => {
            collect_aliases_from_query(subquery, flags);
            if let Some(alias) = alias {
                flags.push(alias.explicit);
            }
        }
        TableFactor::Table { alias, .. }
        | TableFactor::Function { alias, .. }
        | TableFactor::TableFunction { alias, .. }
        | TableFactor::OpenJsonTable { alias, .. }
        | TableFactor::JsonTable { alias, .. } => {
            if let Some(alias) = alias {
                flags.push(alias.explicit);
            }
        }
        TableFactor::UNNEST { alias, .. } => {
            if let Some(alias) = alias {
                flags.push(alias.explicit);
            }
        }
        TableFactor::NestedJoin {
            table_with_joins,
            alias,
        } => {
            collect_aliases_from_table_with_joins(table_with_joins, flags);
            if let Some(alias) = alias {
                flags.push(alias.explicit);
            }
        }
        TableFactor::Pivot { table, alias, .. }
        | TableFactor::Unpivot { table, alias, .. }
        | TableFactor::MatchRecognize { table, alias, .. } => {
            collect_aliases_from_table_factor(table, flags);
            if let Some(alias) = alias {
                flags.push(alias.explicit);
            }
        }
        TableFactor::XmlTable { alias, .. } | TableFactor::SemanticView { alias, .. } => {
            if let Some(alias) = alias {
                flags.push(alias.explicit);
            }
        }
    }
}

fn select_dialect(kind: DialectKind) -> Box<dyn Dialect> {
    match kind {
        DialectKind::Ansi => Box::new(AnsiDialect {}),
        DialectKind::Dremio => Box::new(GenericDialect {}),
    }
}

fn normalize_overescaped_literals(input: &str) -> &str {
    // Currently no normalization needed, just return the input reference
    // to avoid unnecessary string allocation
    input
}

fn strip_dremio_version_clause(stmt: &str) -> ParsedStatementData {
    let re = Regex::new(
        r#"(?is)\s+AT\s+(BRANCH|TAG|REF|COMMIT)\s+([^\s;]+)(?:\s+AS\s+OF\s+TIMESTAMP\s+('[^']*'|"[^"]*"|[^\s;]+))?"#,
    )
    .expect("regex");

    if let Some(mat) = re.find(stmt) {
        let captures = re.captures(stmt);
        let mut version = DremioVersionClause {
            at: None,
            as_of_timestamp: None,
        };

        if let Some(caps) = captures {
            let kind = caps.get(1).map(|m| m.as_str().to_uppercase());
            let target = caps.get(2).map(|m| m.as_str().to_string());
            let as_of = caps.get(3).map(|m| m.as_str().to_string());

            if let (Some(kind), Some(target)) = (kind, target) {
                version.at = match kind.as_str() {
                    "BRANCH" => Some(VersionSelector::Branch(target)),
                    "TAG" => Some(VersionSelector::Tag(target)),
                    "REF" => Some(VersionSelector::Ref(target)),
                    "COMMIT" => Some(VersionSelector::Commit(target)),
                    _ => None,
                };
            }
            if let Some(ts) = as_of {
                version.as_of_timestamp = Some(ts);
            }
        }

        let boundary_re = Regex::new(
            r"(?is)\b(WHERE|GROUP\s+BY|HAVING|QUALIFY|WINDOW|ORDER\s+BY|LIMIT|OFFSET|JOIN|INNER|LEFT|RIGHT|FULL|CROSS|UNION|EXCEPT|INTERSECT)\b",
        )
        .expect("boundary regex");
        let after_clause = &stmt[mat.end()..];
        let boundary_pos = boundary_re
            .find(after_clause)
            .map(|m| mat.end() + m.start())
            .unwrap_or(stmt.len());

        // Pre-allocate with estimated capacity to avoid reallocation
        let capacity = mat.start() + 1 + (stmt.len() - boundary_pos);
        let mut stripped = String::with_capacity(capacity);
        stripped.push_str(&stmt[..mat.start()]);
        stripped.push(' ');
        stripped.push_str(&stmt[boundary_pos..]);

        ParsedStatementData {
            sql: stripped.trim().to_string(),
            version: Some(version),
        }
    } else {
        ParsedStatementData {
            sql: stmt.to_string(),
            version: None,
        }
    }
}

fn parse_dremio_command(raw: &str) -> Option<DremioCommand> {
    let trimmed = raw.trim();
    let normalized = collapse_spaces(trimmed);
    let lower = normalized.to_lowercase();

    if let Some(cmd) = parse_alter_table_reflection(trimmed) {
        return Some(DremioCommand::AlterTableReflection(cmd));
    }

    if let Some(rest) = trimmed
        .strip_prefix("USE ")
        .or_else(|| trimmed.strip_prefix("use "))
    {
        return Some(DremioCommand::Use {
            path: rest.trim().to_string(),
        });
    }

    if lower.starts_with("create folder") {
        let rest = normalized["create folder".len()..].trim();
        let (if_not_exists, path) = if rest.to_lowercase().starts_with("if not exists") {
            let path = rest["if not exists".len()..].trim().to_string();
            (true, path)
        } else {
            (false, rest.to_string())
        };
        return Some(DremioCommand::CreateFolder {
            if_not_exists,
            path,
        });
    }

    // Check for branch/tag commands (create/alter/drop/merge branch/tag)
    // Format is "{verb} {kind} " with trailing space
    for (verb, prefix) in [
        ("create", "create branch "),
        ("create", "create tag "),
        ("alter", "alter branch "),
        ("alter", "alter tag "),
        ("drop", "drop branch "),
        ("drop", "drop tag "),
        ("merge", "merge branch "),
        ("merge", "merge tag "),
    ] {
        if lower.starts_with(prefix) {
            let kind = if prefix.contains("branch") {
                "branch"
            } else {
                "tag"
            };
            let rest = normalized[prefix.len()..].trim().to_string();
            return Some(DremioCommand::BranchTag {
                verb: verb.to_string(),
                kind: kind.to_string(),
                rest,
            });
        }
    }

    // Check for show commands
    for (kind, prefix) in [
        ("branches", "show branches"),
        ("tags", "show tags"),
        ("logs", "show logs"),
        ("reflections", "show reflections"),
    ] {
        if lower.starts_with(prefix) {
            let rest = normalized[prefix.len()..].trim().to_string();
            return Some(DremioCommand::Show {
                kind: kind.to_string(),
                rest,
            });
        }
    }

    if lower.starts_with("vacuum catalog") {
        let rest = normalized["vacuum catalog".len()..].trim().to_string();
        return Some(DremioCommand::VacuumCatalog { rest });
    }

    // Check for pipe commands
    for (verb, prefix) in [
        ("create", "create pipe"),
        ("alter", "alter pipe"),
        ("describe", "describe pipe"),
        ("drop", "drop pipe"),
    ] {
        if lower.starts_with(prefix) {
            let rest = normalized[prefix.len()..].trim().to_string();
            return Some(DremioCommand::Pipe {
                verb: verb.to_string(),
                rest,
            });
        }
    }

    // Check for table maintenance commands
    for (verb, prefix) in [
        ("optimize", "optimize table"),
        ("vacuum", "vacuum table"),
        ("rollback", "rollback table"),
    ] {
        if lower.starts_with(prefix) {
            let rest = normalized[prefix.len()..].trim().to_string();
            return Some(DremioCommand::TableMaintenance {
                verb: verb.to_string(),
                rest,
            });
        }
    }

    if lower.starts_with("alter pds") {
        let rest = normalized["alter pds".len()..].trim().to_string();
        return Some(DremioCommand::AlterPds { rest });
    }

    if starts_with_command_prefix(&lower, "copy into table") {
        let rest = normalized["copy into table".len()..].trim().to_string();
        return Some(DremioCommand::TableMaintenance {
            verb: "COPY INTO TABLE".to_string(),
            rest,
        });
    }

    if starts_with_command_prefix(&lower, "copy into") {
        let rest = normalized["copy into".len()..].trim().to_string();
        return Some(DremioCommand::TableMaintenance {
            verb: "COPY INTO".to_string(),
            rest,
        });
    }

    if lower.starts_with("analyze table") {
        let rest = normalized["analyze table".len()..].trim().to_string();
        return Some(DremioCommand::AnalyzeTable { rest });
    }

    if starts_with_command_prefix(&lower, "show tblproperties") {
        let rest = normalized["show tblproperties".len()..].trim().to_string();
        return Some(DremioCommand::ShowTableProperties { rest });
    }

    if lower.starts_with("show table properties") {
        let rest = normalized["show table properties".len()..]
            .trim()
            .to_string();
        return Some(DremioCommand::ShowTableProperties { rest });
    }

    // Check for acceleration management commands
    for (verb, prefix) in [
        ("create", "create acceleration"),
        ("alter", "alter acceleration"),
        ("drop", "drop acceleration"),
        ("describe", "describe acceleration"),
        ("refresh", "refresh acceleration"),
    ] {
        if lower.starts_with(prefix) {
            let rest = normalized[prefix.len()..].trim().to_string();
            return Some(DremioCommand::AccelerationManage {
                verb: verb.to_string(),
                rest,
            });
        }
    }

    if lower.starts_with("acceleration") {
        let rest = normalized["acceleration".len()..].trim().to_string();
        return Some(DremioCommand::Acceleration { rest });
    }

    // Check for reflection commands
    for (verb, prefix) in [
        ("create", "create reflection"),
        ("alter", "alter reflection"),
        ("drop", "drop reflection"),
        ("describe", "describe reflection"),
        ("refresh", "refresh reflection"),
    ] {
        if lower.starts_with(prefix) {
            let rest = normalized[prefix.len()..].trim().to_string();
            return Some(DremioCommand::Reflection {
                verb: verb.to_string(),
                rest,
            });
        }
    }

    for kind in ["roles", "users"] {
        if starts_with_command_prefix(&lower, kind) {
            let rest = trimmed[kind.len()..].trim().to_string();
            return Some(DremioCommand::RolesUsers {
                kind: kind.to_string(),
                rest,
            });
        }
    }

    if starts_with_command_prefix(&lower, "row column policies") {
        let rest = normalized["row column policies".len()..].trim().to_string();
        return Some(DremioCommand::RowColumnPolicies { rest });
    }

    if starts_with_command_prefix(&lower, "row-column policies") {
        let rest = normalized["row-column policies".len()..].trim().to_string();
        return Some(DremioCommand::RowColumnPolicies { rest });
    }

    for (head, prefix) in [
        (&["ALTER", "FOLDER"][..], "alter folder"),
        (&["ALTER", "SOURCE"][..], "alter source"),
        (&["ALTER", "SPACE"][..], "alter space"),
        (&["ALTER", "VIEW"][..], "alter view"),
        (&["ALTER", "TABLE"][..], "alter table"),
        (&["GRANT"][..], "grant"),
        (&["REVOKE"][..], "revoke"),
        (&["CREATE", "ROLE"][..], "create role"),
        (&["DROP", "ROLE"][..], "drop role"),
        (&["GRANT", "ROLE"][..], "grant role"),
        (&["REVOKE", "ROLE"][..], "revoke role"),
        (&["CREATE", "USER"][..], "create user"),
        (&["ALTER", "USER"][..], "alter user"),
        (&["DROP", "USER"][..], "drop user"),
        (&["SHOW", "FUNCTIONS"][..], "show functions"),
        (&["CREATE", "FUNCTION"][..], "create function"),
        (&["DROP", "FUNCTION"][..], "drop function"),
        (&["DESCRIBE", "FUNCTION"][..], "describe function"),
        (&["ALTER", "ROLE"][..], "alter role"),
    ] {
        if starts_with_command_prefix(&lower, prefix) {
            let rest = normalized[prefix.len()..].trim().to_string();
            return Some(DremioCommand::Generic {
                head: head.iter().map(|s| (*s).to_string()).collect(),
                rest,
            });
        }
    }

    // Check for queue/tag commands (set/reset queue/tag)
    for (verb, prefix) in [
        ("set", "set queue"),
        ("set", "set tag"),
        ("reset", "reset queue"),
        ("reset", "reset tag"),
    ] {
        if lower.starts_with(prefix) {
            let kind = if prefix.contains("queue") {
                "queue"
            } else {
                "tag"
            };
            let rest = normalized[prefix.len()..].trim().to_string();
            return Some(DremioCommand::QueueTag {
                verb: verb.to_string(),
                kind: kind.to_string(),
                rest,
            });
        }
    }

    None
}

fn parse_alter_table_reflection(raw: &str) -> Option<AlterTableReflectionCommand> {
    let re = Regex::new(
        r#"(?is)^alter\s+table\s+(?P<table>.+?)\s+create\s+(?:(?P<kind>raw|aggregate)\s+)?reflection\s+(?P<name>("([^"]|"")*"|[^\s(]+))\s*(?P<rest>.*)$"#,
    )
    .ok()?;

    let caps = re.captures(raw)?;
    let table = caps.name("table")?.as_str().trim().to_string();
    let kind = caps
        .name("kind")
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty());
    let name = caps
        .name("name")
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty());
    let rest = caps.name("rest").map(|m| m.as_str()).unwrap_or("").trim();

    let clauses = split_reflection_option_clauses(rest);

    Some(AlterTableReflectionCommand {
        table,
        kind,
        name,
        clauses,
    })
}

fn split_reflection_option_clauses(rest: &str) -> Vec<(String, String)> {
    let clause_re = Regex::new(
        r"(?is)\b(USING DISPLAY|USING DIMENSIONS|USING MEASURES|LOCALSORT BY|PARTITION BY|DISTRIBUTE BY|SORT BY|USING)\b",
    )
    .expect("reflection clause regex");

    let mut matches: Vec<_> = clause_re.find_iter(rest).collect();
    if matches.is_empty() {
        return Vec::new();
    }

    // keep longest keyword when overlaps (e.g., USING DISPLAY vs USING)
    matches.sort_by_key(|m| (m.start(), usize::MAX - m.as_str().len()));
    matches.dedup_by(|a, b| a.start() == b.start());
    matches.sort_by_key(|m| m.start());

    let mut clauses = Vec::new();
    for (idx, mat) in matches.iter().enumerate() {
        let end = matches
            .get(idx + 1)
            .map(|next| next.start())
            .unwrap_or_else(|| rest.len());
        let label = mat.as_str().trim().to_string();
        let value = rest[mat.end()..end].trim();
        clauses.push((label, value.to_string()));
    }

    clauses
}

struct ParsedStatementData {
    sql: String,
    version: Option<DremioVersionClause>,
}

fn collapse_spaces(input: &str) -> String {
    // Avoid intermediate Vec allocation for better performance
    // Use conservative capacity estimate since we remove whitespace
    let mut result = String::with_capacity(input.len());
    let mut prev_was_space = true; // Start true to skip leading spaces
    for c in input.chars() {
        if c.is_whitespace() {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            result.push(c);
            prev_was_space = false;
        }
    }
    // Remove trailing space if any
    if result.ends_with(' ') {
        result.pop();
    }
    result
}

fn starts_with_command_prefix(input_lower: &str, prefix: &str) -> bool {
    input_lower == prefix || input_lower.starts_with(&format!("{prefix} "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlparser::ast::{SetExpr, Statement};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn reference_command_fixture_paths() -> Vec<PathBuf> {
        let base = Path::new("fixtures/dremio/reference-commands");
        let mut files = fs::read_dir(base)
            .expect("read fixtures/dremio/reference-commands")
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("sql"))
            .collect::<Vec<_>>();
        files.sort();
        files
    }

    #[test]
    fn parses_simple_select() {
        let sql = "SELECT a, b FROM t;";
        let stmts = parse_sql(sql, DialectKind::Ansi).expect("parse");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(
            stmts[0],
            ParsedStatement::Sql {
                has_semicolon: true,
                ..
            }
        ));
    }

    #[test]
    fn parses_cte_and_join() {
        let sql = r#"
        WITH cte AS (
            SELECT id, name FROM users
        )
        SELECT u.id, o.total
        FROM cte AS u
        JOIN orders AS o ON o.user_id = u.id
        "#;

        let stmts = parse_sql(sql, DialectKind::Ansi).expect("parse");
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn parses_basic_ddl() {
        let sql = r#"
        CREATE TABLE example (id INT, name TEXT);
        CREATE VIEW v_example AS SELECT id, name FROM example;
        "#;

        let stmts = parse_sql(sql, DialectKind::Ansi).expect("parse");
        assert_eq!(stmts.len(), 2);
        assert!(matches!(
            stmts[0],
            ParsedStatement::Sql {
                has_semicolon: true,
                ..
            }
        ));
        assert!(matches!(
            stmts[1],
            ParsedStatement::Sql {
                has_semicolon: true,
                ..
            }
        ));
    }

    #[test]
    fn falls_back_to_raw_when_parsing_fails() {
        let sql = "unknown_verb select field from thing where flag=true;";
        let stmts = parse_sql(sql, DialectKind::Ansi).expect("parse");
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            ParsedStatement::Raw { sql, has_semicolon } => {
                assert_eq!(sql, "unknown_verb select field from thing where flag=true");
                assert!(has_semicolon);
            }
            other => panic!("expected raw fallback, got {other:?}"),
        }
    }

    #[test]
    fn strict_mode_propagates_parse_errors() {
        let sql = "unknown_verb select field from thing where flag=true;";
        let res = parse_sql_with_options(sql, DialectKind::Ansi, ParseOptions { strict: true });
        assert!(res.is_err());
    }

    #[test]
    fn parses_dremio_path_with_quoted_segments() {
        let sql = r#"SELECT * FROM Samples."samples.dremio.com"."NYC-taxi-trips""#;
        let stmts = parse_sql(sql, DialectKind::Dremio).expect("parse dremio path");
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn parses_dremio_reflection_command() {
        let sql = "create reflection my_reflection using table foo";
        let stmts = parse_sql(sql, DialectKind::Dremio).expect("parse reflection");
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn tracks_semicolons_across_dremio_statements() {
        let sql = "use foo; select * from bar";
        let stmts = parse_sql(sql, DialectKind::Dremio).expect("parse dremio");
        assert_eq!(stmts.len(), 2);
        assert!(matches!(
            stmts[0],
            ParsedStatement::Command {
                has_semicolon: true,
                ..
            }
        ));
        assert!(matches!(
            stmts[1],
            ParsedStatement::Sql {
                has_semicolon: false,
                ..
            }
        ));
    }

    #[test]
    fn parses_dremio_show_reflections() {
        let sql = "show reflections in my_space";
        let stmts = parse_sql(sql, DialectKind::Dremio).expect("parse show reflections");
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn parses_dremio_create_folder() {
        let sql = "\
CREATE FOLDER IF NOT EXISTS demoCatalog.sales;
CREATE FOLDER IF NOT EXISTS demoCatalog.shared;
CREATE FOLDER IF NOT EXISTS demoCatalog.sales.staging;
CREATE FOLDER IF NOT EXISTS demoCatalog.sales.curated;";
        let stmts = parse_sql(sql, DialectKind::Dremio).expect("parse create folder");
        assert_eq!(stmts.len(), 4);
        assert!(stmts.iter().all(|s| matches!(
            s,
            ParsedStatement::Command {
                cmd: DremioCommand::CreateFolder { .. },
                has_semicolon: true
            }
        )));
    }

    #[test]
    fn captures_relation_alias_as_usage() {
        let sql = r#"SELECT * FROM "ltv_daily" m LEFT JOIN analytics_space."dim_customers" d ON d.customer_id = m.customer_id"#;
        let stmts = parse_sql(sql, DialectKind::Dremio).expect("parse aliases");
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            ParsedStatement::Sql {
                relation_alias_has_as,
                ..
            } => assert_eq!(relation_alias_has_as, &vec![false, false]),
            _ => panic!("expected SQL statement"),
        }
    }

    #[test]
    fn captures_alias_with_command_prefix() {
        let sql = "\
USE analytics_space;
SELECT *
FROM ltv_daily m
LEFT JOIN analytics_space.dim_customers d ON d.customer_id = m.customer_id;";
        let stmts = parse_sql(sql, DialectKind::Dremio).expect("parse command + select");
        assert_eq!(stmts.len(), 2);
        match &stmts[1] {
            ParsedStatement::Sql {
                relation_alias_has_as,
                ..
            } => assert_eq!(relation_alias_has_as, &vec![false, false]),
            _ => panic!("expected SQL statement"),
        }
    }

    #[test]
    fn parses_dremio_alter_pds_commands() {
        let sql = "\
ALTER PDS demo_source.sales.\"chargeback_files\" REFRESH METADATA FORCE UPDATE;
ALTER PDS demo_source.sales.\"chargebacks\" REFRESH METADATA FORCE UPDATE;
ALTER PDS demo_source.sales.\"refund_send_info\" REFRESH METADATA FORCE UPDATE;";
        let stmts = parse_sql(sql, DialectKind::Dremio).expect("parse alter pds");
        assert_eq!(stmts.len(), 3);
        assert!(stmts.iter().all(|s| matches!(
            s,
            ParsedStatement::Command {
                cmd: DremioCommand::AlterPds { .. },
                has_semicolon: true
            }
        )));
    }

    #[test]
    fn captures_aliases_in_ctas() {
        let sql = "\
CREATE TABLE arctic_catalog.analytics_space.daily_revenue_by_country AS
SELECT
    cast(order_ts AS date) AS order_date,
    c.country,
    sum(o.order_amount) AS total_revenue
FROM arctic_catalog.analytics_space.fact_orders o
JOIN arctic_catalog.analytics_space.dim_customers c ON c.customer_id = o.customer_id;";
        let stmts = parse_sql(sql, DialectKind::Dremio).expect("parse CTAS with aliases");
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            ParsedStatement::Sql {
                relation_alias_has_as,
                ..
            } => assert_eq!(relation_alias_has_as, &vec![false, false]),
            _ => panic!("expected SQL statement"),
        }
    }

    #[test]
    fn captures_alias_in_joined_subquery_without_as() {
        let sql = "SELECT * FROM base LEFT JOIN (SELECT customer_id, max(order_ts) AS last_order_ts FROM fact_orders GROUP BY customer_id) last_o ON last_o.customer_id = base.customer_id";
        let stmts = parse_sql(sql, DialectKind::Ansi).expect("parse subquery join alias");
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            ParsedStatement::Sql {
                relation_alias_has_as,
                ..
            } => assert_eq!(relation_alias_has_as, &vec![false]),
            _ => panic!("expected SQL statement"),
        }
    }

    #[test]
    fn parses_dremio_refresh_acceleration_command() {
        let sql = "refresh acceleration my_reflection with (refresh = 'auto')";
        let stmts = parse_sql(sql, DialectKind::Dremio).expect("parse refresh acceleration");
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn parses_dremio_alter_reflection_with_spacing_and_using() {
        let sql = r#"
        alter  reflection  analytics_space.daily_revenue_by_country_reflection
        using
            display name 'Daily revenue by country reflection'
            partition by (order_date)
            distribute by (country)
            sort by (order_date, country)
        ;
        "#;
        let stmts = parse_sql(sql, DialectKind::Dremio).expect("parse alter reflection");
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn parses_dremio_parenthesized_ctas_statement() {
        let sql = r#"
CREATE TABLE demoCatalog.reporting."tables".orders_partitioned AS (
SELECT
    o.id AS order_id,
    CURRENT_TIMESTAMP AS sync_time
FROM demoCatalog.reporting."tables"."orders" o
);
        "#;
        let stmts = parse_sql(sql, DialectKind::Dremio).expect("parse");
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            ParsedStatement::Sql {
                stmt,
                relation_alias_has_as,
                ..
            } => {
                assert!(
                    !relation_alias_has_as.is_empty(),
                    "expected relation alias metadata"
                );
                match stmt.as_ref() {
                    Statement::CreateTable(create_table) => {
                        let q = create_table
                            .query
                            .as_ref()
                            .expect("expected CTAS query to be present");
                        assert!(matches!(
                            q.body.as_ref(),
                            SetExpr::Select(_) | SetExpr::Query(_)
                        ));
                    }
                    other => panic!("unexpected statement: {other:?}"),
                }
            }
            other => panic!("unexpected parsed statement: {other:?}"),
        }
    }

    #[test]
    fn parses_copy_into_without_table_keyword() {
        let sql = "COPY INTO my_space.my_table FROM '@/files' FILE_FORMAT 'csv'";
        let stmts = parse_sql(sql, DialectKind::Dremio).expect("parse copy into");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(
            stmts[0],
            ParsedStatement::Command {
                cmd: DremioCommand::TableMaintenance { .. },
                ..
            }
        ));
    }

    #[test]
    fn parses_all_dremio_reference_command_fixtures_in_strict_mode() {
        let files = reference_command_fixture_paths();
        assert_eq!(files.len(), 57, "expected 57 reference command fixtures");

        for path in files {
            let sql = fs::read_to_string(&path).expect("read fixture");
            parse_sql_with_options(&sql, DialectKind::Dremio, ParseOptions { strict: true })
                .unwrap_or_else(|err| panic!("strict parse failed for {:?}: {err}", path));
        }
    }
}
