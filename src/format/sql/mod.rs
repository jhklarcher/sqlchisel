use anyhow::{Context, Result};
use sqlparser::ast::{
    ArrayElemTypeDef, CastFormat, CastKind, ColumnDef, CreateTableOptions, DataType, DateTimeField,
    Expr, Function, GroupByExpr, HiveDistributionStyle, HiveFormat, Insert, Interval, ObjectName,
    ObjectNamePart, OneOrManyWithParens, OrderByExpr, Query, Select, SetExpr, Statement,
    TableConstraint, TableFactor, TableWithJoins, TypedString, Value, ValueWithSpan, ViewColumnDef,
    WindowFrame, WindowSpec, WindowType, WrappedCollection,
};
use sqlparser::dialect::{AnsiDialect, Dialect, GenericDialect};

use crate::config::{DialectKind, FormatterConfig, KeywordCase};
use crate::format::doc::Doc;
use crate::format::printer::{format_doc, PrintConfig};
use crate::parser::{parse_sql_with_options, DremioVersionClause, ParseOptions, ParsedStatement};

mod case;
mod comments;
mod dremio;
mod expr;
mod from_join;
mod jinja;
mod layout;
mod literals;
mod query;
mod raw;
mod select;

use self::comments::{extract_comments, reattach_comments};
use self::dremio::{format_dremio_command, format_dremio_version_clause};
use self::expr::format_expr;
use self::from_join::{
    format_boolean_clause, format_comma_clause, format_comma_clause_per_line, format_from,
};
use self::jinja::{contains_jinja_markers, preserve_jinja_expressions, restore_jinja_expressions};
use self::layout::choose_layout;
use self::literals::{preserve_string_literals, restore_string_literals};
use self::query::{format_query, format_query_with_layout_preference};
use self::raw::format_raw_sql;
use self::select::format_select;

struct RelationAliasTracker<'a> {
    flags: &'a [bool],
    index: usize,
}

impl<'a> RelationAliasTracker<'a> {
    fn new(flags: &'a [bool]) -> Self {
        Self { flags, index: 0 }
    }

    fn next(&mut self) -> Option<bool> {
        if self.index < self.flags.len() {
            let value = self.flags[self.index];
            self.index += 1;
            Some(value)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectLayout {
    Inline,
    PerLine,
}

pub fn format_sql(input: &str, cfg: &FormatterConfig) -> Result<String> {
    format_sql_impl(input, cfg, true)
}

fn format_sql_impl(input: &str, cfg: &FormatterConfig, allow_jinja_blocks: bool) -> Result<String> {
    if allow_jinja_blocks && contains_jinja_markers(input) {
        return Ok(input.to_string());
    }
    let comments = extract_comments(input, &cfg.dialect)?;
    let (patched_input, jinja_exprs) = if allow_jinja_blocks {
        preserve_jinja_expressions(input)
    } else {
        (input.to_string(), Vec::new())
    };
    let (patched_input, literals) = preserve_string_literals(&patched_input);
    let stmts = parse_sql_with_options(
        &patched_input,
        cfg.dialect,
        ParseOptions { strict: cfg.strict },
    )
    .context("parse failed")?;
    let mut docs = Vec::new();
    for (idx, stmt) in stmts.iter().enumerate() {
        if idx > 0 {
            docs.push(Doc::Line);
            docs.push(Doc::Line);
        }
        let (mut doc, has_semicolon) = match stmt {
            ParsedStatement::Sql {
                stmt,
                version,
                has_semicolon,
                relation_alias_has_as,
            } => (
                format_statement(
                    stmt.as_ref(),
                    cfg,
                    version.clone(),
                    &mut RelationAliasTracker::new(relation_alias_has_as),
                )?,
                *has_semicolon,
            ),
            ParsedStatement::Command { cmd, has_semicolon } => {
                (format_dremio_command(cmd, cfg), *has_semicolon)
            }
            ParsedStatement::Raw { sql, has_semicolon } => {
                (format_raw_sql(sql, cfg)?, *has_semicolon)
            }
        };
        if has_semicolon {
            doc = Doc::Group(vec![doc, Doc::Text(";".into())]);
        }
        docs.push(doc);
    }
    let print_cfg = PrintConfig {
        line_length: cfg.line_length,
        indent_width: cfg.indent_width,
    };
    let rendered = format_doc(&Doc::Concat(docs), &print_cfg);
    let restored = restore_string_literals(rendered, literals);
    let restored = restore_jinja_expressions(restored, jinja_exprs);
    reattach_comments(restored, comments, &cfg.dialect)
}

fn format_statement(
    stmt: &Statement,
    cfg: &FormatterConfig,
    version: Option<DremioVersionClause>,
    alias_tracker: &mut RelationAliasTracker,
) -> Result<Doc> {
    match stmt {
        Statement::Query(query) => format_query(query, cfg, version, alias_tracker),
        Statement::CreateTable(create_table) => {
            let simple_layout = create_table.query.is_none()
                && create_table.like.is_none()
                && create_table.clone.is_none()
                && create_table.version.is_none()
                && matches!(create_table.table_options, CreateTableOptions::None)
                && create_table.file_format.is_none()
                && create_table.location.is_none()
                && create_table.comment.is_none()
                && create_table.on_commit.is_none()
                && create_table.on_cluster.is_none()
                && create_table.primary_key.is_none()
                && create_table.order_by.is_none()
                && create_table.partition_by.is_none()
                && create_table.cluster_by.is_none()
                && create_table.clustered_by.is_none()
                && create_table.inherits.is_none()
                && create_table.partition_of.is_none()
                && create_table.for_values.is_none()
                && matches!(create_table.hive_distribution, HiveDistributionStyle::NONE)
                && create_table
                    .hive_formats
                    .as_ref()
                    .is_none_or(hive_format_is_empty)
                && !create_table.without_rowid
                && !create_table.copy_grants
                && !create_table.strict
                && !create_table.dynamic
                && !create_table.transient
                && !create_table.volatile
                && !create_table.iceberg
                && create_table.enable_schema_evolution.is_none()
                && create_table.change_tracking.is_none()
                && create_table.data_retention_time_in_days.is_none()
                && create_table.max_data_extension_time_in_days.is_none()
                && create_table.default_ddl_collation.is_none()
                && create_table.with_aggregation_policy.is_none()
                && create_table.with_row_access_policy.is_none()
                && create_table.with_tags.is_none()
                && create_table.external_volume.is_none()
                && create_table.base_location.is_none()
                && create_table.catalog.is_none()
                && create_table.catalog_sync.is_none()
                && create_table.storage_serialization_policy.is_none()
                && create_table.target_lag.is_none()
                && create_table.warehouse.is_none()
                && create_table.refresh_mode.is_none()
                && create_table.initialize.is_none()
                && !create_table.require_user;

            let can_format_ctas = create_table.query.is_some()
                && create_table.columns.is_empty()
                && create_table.constraints.is_empty()
                && create_table.like.is_none()
                && create_table.clone.is_none()
                && create_table.version.is_none()
                && matches!(create_table.table_options, CreateTableOptions::None)
                && create_table.file_format.is_none()
                && create_table.location.is_none()
                && create_table.comment.is_none()
                && create_table.on_commit.is_none()
                && create_table.on_cluster.is_none()
                && create_table.primary_key.is_none()
                && create_table.clustered_by.is_none()
                && create_table.inherits.is_none()
                && create_table.partition_of.is_none()
                && create_table.for_values.is_none()
                && matches!(create_table.hive_distribution, HiveDistributionStyle::NONE)
                && create_table
                    .hive_formats
                    .as_ref()
                    .is_none_or(hive_format_is_empty)
                && !create_table.without_rowid
                && !create_table.copy_grants
                && !create_table.strict
                && !create_table.dynamic
                && !create_table.transient
                && !create_table.volatile
                && !create_table.iceberg
                && create_table.enable_schema_evolution.is_none()
                && create_table.change_tracking.is_none()
                && create_table.data_retention_time_in_days.is_none()
                && create_table.max_data_extension_time_in_days.is_none()
                && create_table.default_ddl_collation.is_none()
                && create_table.with_aggregation_policy.is_none()
                && create_table.with_row_access_policy.is_none()
                && create_table.with_tags.is_none()
                && create_table.external_volume.is_none()
                && create_table.base_location.is_none()
                && create_table.catalog.is_none()
                && create_table.catalog_sync.is_none()
                && create_table.storage_serialization_policy.is_none()
                && create_table.target_lag.is_none()
                && create_table.warehouse.is_none()
                && create_table.refresh_mode.is_none()
                && create_table.initialize.is_none()
                && !create_table.require_user;

            if simple_layout {
                Ok(format_create_table(
                    &create_table.name,
                    &create_table.columns,
                    &create_table.constraints,
                    TableFormatOptions {
                        if_not_exists: create_table.if_not_exists,
                        or_replace: create_table.or_replace,
                        temporary: create_table.temporary,
                        external: create_table.external,
                        global: create_table.global,
                    },
                    cfg,
                ))
            } else if can_format_ctas {
                format_create_table_with_query(
                    &create_table.name,
                    create_table
                        .query
                        .as_ref()
                        .expect("query exists for ctas")
                        .as_ref(),
                    TableFormatOptions {
                        if_not_exists: create_table.if_not_exists,
                        or_replace: create_table.or_replace,
                        temporary: create_table.temporary,
                        external: create_table.external,
                        global: create_table.global,
                    },
                    cfg,
                    alias_tracker,
                    CreateTableLayout {
                        order_by: create_table.order_by.as_ref(),
                        partition_by: create_table.partition_by.as_deref(),
                        cluster_by: create_table.cluster_by.as_ref(),
                    },
                )
            } else {
                Ok(Doc::Text(stringify_with_alias_styles(stmt, alias_tracker)))
            }
        }
        Statement::CreateView(create_view) => {
            let simple_layout = matches!(create_view.options, CreateTableOptions::None)
                && create_view.cluster_by.is_empty()
                && !create_view.with_no_schema_binding
                && !create_view.or_alter
                && !create_view.secure
                && create_view.comment.is_none()
                && create_view.to.is_none()
                && create_view.params.is_none();

            if simple_layout {
                format_create_view(
                    &create_view.name,
                    &create_view.columns,
                    create_view.query.as_ref(),
                    CreateViewOptions {
                        or_replace: create_view.or_replace,
                        materialized: create_view.materialized,
                        if_not_exists: create_view.if_not_exists,
                        temporary: create_view.temporary,
                    },
                    cfg,
                    alias_tracker,
                )
            } else {
                Ok(Doc::Text(stringify_with_alias_styles(stmt, alias_tracker)))
            }
        }
        Statement::Insert(insert) => format_insert(insert, cfg, alias_tracker),
        other => Ok(Doc::Text(stringify_with_alias_styles(other, alias_tracker))),
    }
}

struct TableFormatOptions {
    if_not_exists: bool,
    or_replace: bool,
    temporary: bool,
    external: bool,
    global: Option<bool>,
}

fn format_create_table(
    name: &ObjectName,
    columns: &[ColumnDef],
    constraints: &[TableConstraint],
    opts: TableFormatOptions,
    cfg: &FormatterConfig,
) -> Doc {
    let head = create_table_head(opts, cfg);
    let mut parts = vec![Doc::Text(head), Doc::Space, Doc::Text(name.to_string())];

    let mut entries: Vec<String> = columns.iter().map(|c| c.to_string()).collect();
    entries.extend(constraints.iter().map(|c| c.to_string()));

    if !entries.is_empty() {
        parts.push(Doc::Space);
        parts.push(format_parenthesized_block(entries));
    }

    Doc::Group(parts)
}

struct CreateTableLayout<'a> {
    order_by: Option<&'a OneOrManyWithParens<Expr>>,
    partition_by: Option<&'a Expr>,
    cluster_by: Option<&'a WrappedCollection<Vec<Expr>>>,
}

fn format_create_table_with_query(
    name: &ObjectName,
    query: &Query,
    opts: TableFormatOptions,
    cfg: &FormatterConfig,
    alias_tracker: &mut RelationAliasTracker,
    layout: CreateTableLayout<'_>,
) -> Result<Doc> {
    let head = create_table_head(opts, cfg);
    let mut parts = vec![Doc::Text(head), Doc::Space, Doc::Text(name.to_string())];
    let mut has_pre_as_clause = false;

    if let Some(expr) = layout.partition_by {
        let items = match expr {
            Expr::Tuple(exprs) => exprs.iter().map(|e| e.to_string()).collect(),
            Expr::Nested(inner) => vec![inner.to_string()],
            other => vec![other.to_string()],
        };
        parts.push(Doc::Line);
        parts.push(format_parenthesized_clause(
            "PARTITION BY",
            items,
            cfg,
            false,
        ));
        has_pre_as_clause = true;
    }

    if let Some(items) = layout.cluster_by {
        match items {
            WrappedCollection::Parentheses(exprs) if !exprs.is_empty() => {
                let exprs = exprs.iter().map(|e| e.to_string()).collect();
                parts.push(Doc::Line);
                parts.push(format_parenthesized_clause("CLUSTER BY", exprs, cfg, false));
                has_pre_as_clause = true;
            }
            WrappedCollection::NoWrapping(exprs) if !exprs.is_empty() => {
                let exprs = exprs.iter().map(|e| e.to_string()).collect();
                parts.push(Doc::Line);
                parts.push(format_comma_clause("CLUSTER BY", exprs, cfg));
                has_pre_as_clause = true;
            }
            _ => {}
        }
    }

    if let Some(items) = layout.order_by {
        if !items.is_empty() {
            let order = items.iter().map(|o| o.to_string()).collect();
            parts.push(Doc::Line);
            parts.push(format_comma_clause("ORDER BY", order, cfg));
            has_pre_as_clause = true;
        }
    }

    if has_pre_as_clause {
        parts.push(Doc::Line);
        parts.push(keyword_doc(cfg, "AS"));
    } else {
        parts.push(Doc::Space);
        parts.push(keyword_doc(cfg, "AS"));
    }

    let prefer_multiline = has_pre_as_clause
        || (cfg.dialect == DialectKind::Dremio && should_prefer_multiline_ctas(query));
    let body =
        format_query_with_layout_preference(query, cfg, None, alias_tracker, prefer_multiline)
            .unwrap_or_else(|_| Doc::Text(query.to_string()));

    parts.push(Doc::Line);
    parts.push(body);

    Ok(Doc::Group(parts))
}

fn should_prefer_multiline_ctas(query: &Query) -> bool {
    match query.body.as_ref() {
        SetExpr::Select(select) => {
            select.projection.len() > 1
                || select.selection.is_some()
                || select.from.iter().any(|rel| !rel.joins.is_empty())
                || !matches!(&select.group_by, GroupByExpr::Expressions(exprs, _) if exprs.is_empty())
                || select.having.is_some()
        }
        _ => true,
    }
}

fn format_insert(
    insert: &Insert,
    cfg: &FormatterConfig,
    alias_tracker: &mut RelationAliasTracker,
) -> Result<Doc> {
    let source = match &insert.source {
        Some(query) => query,
        None => return Ok(Doc::Text(Statement::Insert(insert.clone()).to_string())),
    };

    let complex = insert.overwrite
        || insert.partitioned.is_some()
        || !insert.after_columns.is_empty()
        || insert.on.is_some()
        || insert.returning.is_some()
        || insert.replace_into
        || insert.priority.is_some()
        || insert.insert_alias.is_some()
        || insert.ignore
        || insert.or.is_some();

    if complex {
        return Ok(Doc::Text(Statement::Insert(insert.clone()).to_string()));
    }

    let mut head = Vec::new();
    head.push(keyword_doc(cfg, "INSERT"));
    if insert.into {
        head.push(Doc::Space);
        head.push(keyword_doc(cfg, "INTO"));
    }
    head.push(Doc::Space);
    head.push(Doc::Text(insert.table.to_string()));

    if let Some(alias) = &insert.table_alias {
        head.push(Doc::Space);
        head.push(Doc::Text(alias.to_string()));
    }

    if !insert.columns.is_empty() {
        let cols: Vec<String> = insert.columns.iter().map(|c| c.to_string()).collect();
        head.push(Doc::Space);
        head.push(format_parenthesized_inline(cols));
    }

    let body = format_query(source, cfg, None, alias_tracker)
        .unwrap_or_else(|_| Doc::Text(source.to_string()));

    Ok(Doc::Group(vec![Doc::Group(head), Doc::Line, body]))
}

fn create_table_head(opts: TableFormatOptions, cfg: &FormatterConfig) -> String {
    let mut head_parts = vec![apply_keyword_case("CREATE", cfg)];
    if opts.or_replace {
        head_parts.push(apply_keyword_case("OR REPLACE", cfg));
    }
    if let Some(scope) = opts.global {
        if scope {
            head_parts.push(apply_keyword_case("GLOBAL", cfg));
        } else {
            head_parts.push(apply_keyword_case("LOCAL", cfg));
        }
    }
    if opts.temporary {
        head_parts.push(apply_keyword_case("TEMPORARY", cfg));
    }
    if opts.external {
        head_parts.push(apply_keyword_case("EXTERNAL", cfg));
    }
    head_parts.push(apply_keyword_case("TABLE", cfg));
    if opts.if_not_exists {
        head_parts.push(apply_keyword_case("IF NOT EXISTS", cfg));
    }
    head_parts.join(" ")
}

struct CreateViewOptions {
    or_replace: bool,
    materialized: bool,
    if_not_exists: bool,
    temporary: bool,
}

fn format_create_view(
    name: &ObjectName,
    columns: &[ViewColumnDef],
    query: &Query,
    opts: CreateViewOptions,
    cfg: &FormatterConfig,
    alias_tracker: &mut RelationAliasTracker,
) -> Result<Doc> {
    let mut head_parts = vec![apply_keyword_case("CREATE", cfg)];
    if opts.or_replace {
        head_parts.push(apply_keyword_case("OR REPLACE", cfg));
    }
    if opts.temporary {
        head_parts.push(apply_keyword_case("TEMPORARY", cfg));
    }
    if opts.materialized {
        head_parts.push(apply_keyword_case("MATERIALIZED", cfg));
    }
    head_parts.push(apply_keyword_case("VIEW", cfg));
    if opts.if_not_exists {
        head_parts.push(apply_keyword_case("IF NOT EXISTS", cfg));
    }

    let head = head_parts.join(" ");
    let mut parts = vec![Doc::Text(head), Doc::Space, Doc::Text(name.to_string())];

    if !columns.is_empty() {
        let cols = columns.iter().map(|c| c.to_string()).collect();
        parts.push(Doc::Space);
        parts.push(format_parenthesized_inline(cols));
    }

    let body = format_query(query, cfg, None, alias_tracker)
        .unwrap_or_else(|_| Doc::Text(query.to_string()));

    parts.push(Doc::Space);
    parts.push(keyword_doc(cfg, "AS"));
    parts.push(Doc::Line);
    parts.push(body);

    Ok(Doc::Group(parts))
}

fn apply_keyword_case(text: &str, cfg: &FormatterConfig) -> String {
    match cfg.keyword_case {
        KeywordCase::Upper => text.to_uppercase(),
        KeywordCase::Lower => text.to_lowercase(),
        KeywordCase::Capitalize => text
            .split_whitespace()
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn keyword_doc(cfg: &FormatterConfig, text: &str) -> Doc {
    Doc::Text(apply_keyword_case(text, cfg))
}

const BUILTIN_FUNCTION_NAMES: &[&str] = &[
    "avg",
    "count",
    "dense_rank",
    "first_value",
    "lag",
    "last_value",
    "lead",
    "max",
    "min",
    "nth_value",
    "rank",
    "row_number",
    "sum",
];

fn is_builtin_function_name(name: &str) -> bool {
    BUILTIN_FUNCTION_NAMES
        .iter()
        .any(|builtin| builtin.eq_ignore_ascii_case(name))
}

fn format_function_call(
    func: &Function,
    inline_limit: usize,
    cfg: &FormatterConfig,
    alias_tracker: &mut RelationAliasTracker,
) -> Doc {
    let mut func = func.clone();
    let over = func.over.take();

    if func.name.0.len() == 1 {
        if let Some(ObjectNamePart::Identifier(ident)) = func.name.0.first_mut() {
            if ident.quote_style.is_none() && is_builtin_function_name(&ident.value) {
                ident.value = apply_keyword_case(&ident.value, cfg);
            }
        }
    }
    let base = Doc::Text(func.to_string());

    if let Some(window) = over {
        let window_doc = format_window_type(&window, inline_limit, cfg, alias_tracker);
        Doc::Group(vec![base, Doc::Space, window_doc])
    } else {
        base
    }
}

fn format_window_type(
    window: &WindowType,
    inline_limit: usize,
    cfg: &FormatterConfig,
    alias_tracker: &mut RelationAliasTracker,
) -> Doc {
    match window {
        WindowType::NamedWindow(name) => Doc::Group(vec![
            keyword_doc(cfg, "OVER"),
            Doc::Space,
            Doc::Text(name.to_string()),
        ]),
        WindowType::WindowSpec(spec) => format_window_spec(spec, inline_limit, cfg, alias_tracker),
    }
}

fn format_window_spec(
    spec: &WindowSpec,
    inline_limit: usize,
    cfg: &FormatterConfig,
    alias_tracker: &mut RelationAliasTracker,
) -> Doc {
    let mut clauses: Vec<Doc> = Vec::new();

    if let Some(name) = &spec.window_name {
        clauses.push(Doc::Text(name.to_string()));
    }

    if !spec.partition_by.is_empty() {
        let exprs: Vec<Doc> = spec
            .partition_by
            .iter()
            .map(|e| format_expr(e, inline_limit, cfg, alias_tracker))
            .collect();
        clauses.push(format_window_clause("PARTITION BY", exprs, cfg));
    }

    if !spec.order_by.is_empty() {
        let exprs: Vec<Doc> = spec
            .order_by
            .iter()
            .map(|o: &OrderByExpr| Doc::Text(o.to_string()))
            .collect();
        clauses.push(format_window_clause("ORDER BY", exprs, cfg));
    }

    if let Some(frame) = &spec.window_frame {
        clauses.push(format_window_frame(frame, inline_limit, cfg, alias_tracker));
    }

    let inline_len = spec.to_string().len();
    let needs_multiline = clauses.len() > 1 || inline_len > inline_limit;

    if needs_multiline {
        let mut body_parts = Vec::new();
        for (idx, clause) in clauses.iter().enumerate() {
            if idx > 0 {
                body_parts.push(Doc::Line);
            }
            body_parts.push(clause.clone());
        }
        let body = Doc::Group(body_parts);
        return Doc::Group(vec![
            keyword_doc(cfg, "OVER"),
            Doc::Space,
            Doc::Group(vec![
                Doc::Text("(".into()),
                Doc::Line,
                Doc::Indent(Box::new(body)),
                Doc::Line,
                Doc::Text(")".into()),
            ]),
        ]);
    }

    let mut inner_parts = Vec::new();
    for (idx, clause) in clauses.iter().enumerate() {
        if idx > 0 {
            inner_parts.push(Doc::Space);
        }
        inner_parts.push(clause.clone());
    }

    let inner = Doc::Group(inner_parts);
    let parens = Doc::Group(vec![Doc::Text("(".into()), inner, Doc::Text(")".into())]);
    Doc::Group(vec![keyword_doc(cfg, "OVER"), Doc::Space, parens])
}

fn format_window_clause(label: &str, items: Vec<Doc>, cfg: &FormatterConfig) -> Doc {
    if items.is_empty() {
        return Doc::Text(String::new());
    }

    let mut list_parts = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        list_parts.push(item.clone());
        if idx + 1 < items.len() {
            list_parts.push(Doc::Text(",".into()));
            list_parts.push(Doc::SoftLine);
        }
    }
    let list = Doc::Group(list_parts);
    Doc::Group(vec![keyword_doc(cfg, label), Doc::Space, list])
}

fn format_window_frame(
    frame: &WindowFrame,
    inline_limit: usize,
    cfg: &FormatterConfig,
    alias_tracker: &mut RelationAliasTracker,
) -> Doc {
    let mut parts = vec![keyword_doc(cfg, &frame.units.to_string())];

    if let Some(end) = &frame.end_bound {
        parts.push(Doc::Space);
        parts.push(keyword_doc(cfg, "BETWEEN"));
        parts.push(Doc::Space);
        parts.push(format_window_frame_bound(
            &frame.start_bound,
            inline_limit,
            cfg,
            alias_tracker,
        ));
        parts.push(Doc::Space);
        parts.push(keyword_doc(cfg, "AND"));
        parts.push(Doc::Space);
        parts.push(format_window_frame_bound(
            end,
            inline_limit,
            cfg,
            alias_tracker,
        ));
    } else {
        parts.push(Doc::Space);
        parts.push(format_window_frame_bound(
            &frame.start_bound,
            inline_limit,
            cfg,
            alias_tracker,
        ));
    }

    Doc::Group(parts)
}

fn format_window_frame_bound(
    bound: &sqlparser::ast::WindowFrameBound,
    inline_limit: usize,
    cfg: &FormatterConfig,
    alias_tracker: &mut RelationAliasTracker,
) -> Doc {
    match bound {
        sqlparser::ast::WindowFrameBound::CurrentRow => Doc::Group(vec![
            keyword_doc(cfg, "CURRENT"),
            Doc::Space,
            keyword_doc(cfg, "ROW"),
        ]),
        sqlparser::ast::WindowFrameBound::Preceding(None) => Doc::Group(vec![
            keyword_doc(cfg, "UNBOUNDED"),
            Doc::Space,
            keyword_doc(cfg, "PRECEDING"),
        ]),
        sqlparser::ast::WindowFrameBound::Following(None) => Doc::Group(vec![
            keyword_doc(cfg, "UNBOUNDED"),
            Doc::Space,
            keyword_doc(cfg, "FOLLOWING"),
        ]),
        sqlparser::ast::WindowFrameBound::Preceding(Some(expr)) => Doc::Group(vec![
            format_expr(expr, inline_limit, cfg, alias_tracker),
            Doc::Space,
            keyword_doc(cfg, "PRECEDING"),
        ]),
        sqlparser::ast::WindowFrameBound::Following(Some(expr)) => Doc::Group(vec![
            format_expr(expr, inline_limit, cfg, alias_tracker),
            Doc::Space,
            keyword_doc(cfg, "FOLLOWING"),
        ]),
    }
}

fn format_value_literal(value: &ValueWithSpan, cfg: &FormatterConfig) -> Doc {
    match &value.value {
        Value::Boolean(flag) => {
            let text = if *flag { "TRUE" } else { "FALSE" };
            Doc::Text(apply_keyword_case(text, cfg))
        }
        _ => Doc::Text(value.to_string()),
    }
}

fn format_data_type(data_type: &DataType, cfg: &FormatterConfig) -> String {
    if data_type_has_custom(data_type) {
        data_type.to_string()
    } else {
        apply_keyword_case(&data_type.to_string(), cfg)
    }
}

fn data_type_has_custom(data_type: &DataType) -> bool {
    match data_type {
        DataType::Custom(_, _) => true,
        DataType::Array(elem) => match elem {
            ArrayElemTypeDef::None => false,
            ArrayElemTypeDef::Parenthesis(inner) => data_type_has_custom(inner),
            ArrayElemTypeDef::SquareBracket(inner, _) | ArrayElemTypeDef::AngleBracket(inner) => {
                data_type_has_custom(inner)
            }
        },
        DataType::Struct(fields, _) => fields
            .iter()
            .any(|field| data_type_has_custom(&field.field_type)),
        _ => false,
    }
}

fn format_typed_string(typed: &TypedString, cfg: &FormatterConfig) -> Doc {
    if typed.uses_odbc_syntax {
        return Doc::Text(typed.to_string());
    }

    let type_text = format_data_type(&typed.data_type, cfg);
    match &typed.value.value {
        Value::SingleQuotedString(value) => {
            let escaped = value.replace('\'', "''");
            Doc::Text(format!("{type_text} '{escaped}'"))
        }
        _ => Doc::Text(format!("{type_text} {}", typed.value)),
    }
}

fn format_cast_expr(
    kind: &CastKind,
    expr: &Expr,
    data_type: &DataType,
    format: &Option<CastFormat>,
    inline_limit: usize,
    cfg: &FormatterConfig,
    alias_tracker: &mut RelationAliasTracker,
) -> Doc {
    let expr_doc = format_expr(expr, inline_limit, cfg, alias_tracker);
    let type_text = format_data_type(data_type, cfg);

    match kind {
        CastKind::Cast | CastKind::TryCast | CastKind::SafeCast => {
            let mut parts = Vec::new();
            let func_name = match kind {
                CastKind::Cast => "CAST",
                CastKind::TryCast => "TRY_CAST",
                CastKind::SafeCast => "SAFE_CAST",
                _ => unreachable!(),
            };

            parts.push(Doc::Text(apply_keyword_case(func_name, cfg)));
            parts.push(Doc::Text("(".into()));
            parts.push(expr_doc);
            parts.push(Doc::Space);
            parts.push(keyword_doc(cfg, "AS"));
            parts.push(Doc::Space);
            parts.push(Doc::Text(type_text));

            if let Some(fmt) = format {
                parts.push(Doc::Space);
                parts.push(keyword_doc(cfg, "FORMAT"));
                parts.push(Doc::Space);
                parts.push(Doc::Text(fmt.to_string()));
            }

            parts.push(Doc::Text(")".into()));
            Doc::Group(parts)
        }
        CastKind::DoubleColon => {
            Doc::Group(vec![expr_doc, Doc::Text("::".into()), Doc::Text(type_text)])
        }
    }
}

fn format_datetime_field(field: &DateTimeField, cfg: &FormatterConfig) -> String {
    match field {
        DateTimeField::Custom(ident) => ident.to_string(),
        DateTimeField::Week(Some(ident)) => {
            format!("{}({ident})", apply_keyword_case("WEEK", cfg))
        }
        DateTimeField::Week(None) => apply_keyword_case("WEEK", cfg),
        _ => apply_keyword_case(&field.to_string(), cfg),
    }
}

fn format_interval_expr(
    interval: &Interval,
    inline_limit: usize,
    cfg: &FormatterConfig,
    alias_tracker: &mut RelationAliasTracker,
) -> Doc {
    let mut parts = vec![
        Doc::Text(apply_keyword_case("INTERVAL", cfg)),
        Doc::Space,
        format_expr(interval.value.as_ref(), inline_limit, cfg, alias_tracker),
    ];

    if let (
        Some(DateTimeField::Second),
        Some(leading_precision),
        Some(fractional_seconds_precision),
    ) = (
        interval.leading_field.as_ref(),
        interval.leading_precision,
        interval.fractional_seconds_precision,
    ) {
        parts.push(Doc::Space);
        parts.push(Doc::Text(format_datetime_field(
            &DateTimeField::Second,
            cfg,
        )));
        parts.push(Doc::Text(format!(
            " ({leading_precision}, {fractional_seconds_precision})"
        )));
        return Doc::Group(parts);
    }

    if let Some(leading_field) = &interval.leading_field {
        parts.push(Doc::Space);
        parts.push(Doc::Text(format_datetime_field(leading_field, cfg)));
    }

    if let Some(leading_precision) = interval.leading_precision {
        parts.push(Doc::Text(format!(" ({leading_precision})")));
    }

    if let Some(last_field) = &interval.last_field {
        parts.push(Doc::Space);
        parts.push(keyword_doc(cfg, "TO"));
        parts.push(Doc::Space);
        parts.push(Doc::Text(format_datetime_field(last_field, cfg)));
    }

    if let Some(fractional_seconds_precision) = interval.fractional_seconds_precision {
        parts.push(Doc::Text(format!(" ({fractional_seconds_precision})")));
    }

    Doc::Group(parts)
}

fn format_unary_predicate(
    inner: &Expr,
    keyword: &str,
    inline_limit: usize,
    cfg: &FormatterConfig,
    alias_tracker: &mut RelationAliasTracker,
) -> Doc {
    Doc::Group(vec![
        format_expr(inner, inline_limit, cfg, alias_tracker),
        Doc::Space,
        keyword_doc(cfg, keyword),
    ])
}

fn format_parenthesized_block(items: Vec<String>) -> Doc {
    Doc::Group(vec![
        Doc::Text("(".into()),
        Doc::Line,
        Doc::Indent(Box::new(join_comma_lines(items))),
        Doc::Line,
        Doc::Text(")".into()),
    ])
}

fn join_comma_lines(items: Vec<String>) -> Doc {
    let mut parts = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        parts.push(Doc::Text(item.clone()));
        if idx + 1 < items.len() {
            parts.push(Doc::Text(",".into()));
            parts.push(Doc::Line);
        }
    }
    Doc::Group(parts)
}

fn format_parenthesized_inline(items: Vec<String>) -> Doc {
    let mut parts = vec![Doc::Text("(".into())];
    for (idx, item) in items.iter().enumerate() {
        if idx > 0 {
            parts.push(Doc::Text(",".into()));
            parts.push(Doc::Space);
        }
        parts.push(Doc::Text(item.clone()));
    }
    parts.push(Doc::Text(")".into()));
    Doc::Group(parts)
}

fn format_parenthesized_clause(
    label: &str,
    items: Vec<String>,
    cfg: &FormatterConfig,
    prefer_multiline: bool,
) -> Doc {
    if items.is_empty() {
        return Doc::Group(vec![
            keyword_doc(cfg, label),
            Doc::Space,
            Doc::Text("()".into()),
        ]);
    }

    let inline_len = label.len()
        + 3
        + items.iter().map(|s| s.len()).sum::<usize>()
        + (items.len().saturating_sub(1) * 2);

    let force_break = prefer_multiline && items.len() > 1;
    if !force_break && inline_len <= cfg.line_length {
        return Doc::Group(vec![
            keyword_doc(cfg, label),
            Doc::Space,
            format_parenthesized_inline(items),
        ]);
    }

    Doc::Group(vec![
        keyword_doc(cfg, label),
        Doc::Space,
        Doc::Text("(".into()),
        Doc::Line,
        Doc::Indent(Box::new(join_comma_lines(items))),
        Doc::Line,
        Doc::Text(")".into()),
    ])
}

fn split_top_level_commas(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_double => {
                current.push(ch);
                if matches!(chars.peek(), Some('\'')) {
                    current.push(chars.next().unwrap());
                } else {
                    in_single = !in_single;
                }
            }
            '"' if !in_single => {
                current.push(ch);
                if matches!(chars.peek(), Some('"')) {
                    current.push(chars.next().unwrap());
                } else {
                    in_double = !in_double;
                }
            }
            '(' if !in_single && !in_double => {
                depth += 1;
                current.push(ch);
            }
            ')' if !in_single && !in_double => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if !in_single && !in_double && depth == 0 => {
                let piece = current.trim();
                if !piece.is_empty() {
                    parts.push(piece.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let tail = current.trim();
    if !tail.is_empty() {
        parts.push(tail.to_string());
    }

    parts
}

fn render_option_content(rest: &str, cfg: &FormatterConfig) -> Doc {
    let needs_break = rest.len() > (cfg.line_length / 2);

    if rest.starts_with('(') && rest.ends_with(')') {
        let inner = &rest[1..rest.len() - 1];
        let parts: Vec<String> = inner
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() > 1 && needs_break {
            let mut docs = Vec::new();
            for (idx, p) in parts.iter().enumerate() {
                docs.push(Doc::Text(p.clone()));
                if idx + 1 < parts.len() {
                    docs.push(Doc::Text(",".into()));
                    docs.push(Doc::Line);
                }
            }
            return Doc::Group(vec![
                Doc::Text("(".into()),
                Doc::Line,
                Doc::Indent(Box::new(Doc::Group(docs))),
                Doc::Line,
                Doc::Text(")".into()),
            ]);
        }
    }

    if needs_break && rest.contains(',') {
        let parts: Vec<String> = rest
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() > 1 {
            let mut docs = Vec::new();
            for (idx, p) in parts.iter().enumerate() {
                docs.push(Doc::Text(p.clone()));
                if idx + 1 < parts.len() {
                    docs.push(Doc::Text(",".into()));
                    docs.push(Doc::Line);
                }
            }
            return Doc::Group(docs);
        }
    }

    Doc::Text(rest.to_string())
}

fn stringify_with_alias_styles(
    stmt: &Statement,
    alias_tracker: &mut RelationAliasTracker,
) -> String {
    let mut text = stmt.to_string();
    strip_relation_aliases_in_statement(&mut text, stmt, alias_tracker);
    text
}

fn strip_relation_aliases_in_statement(
    text: &mut String,
    stmt: &Statement,
    alias_tracker: &mut RelationAliasTracker,
) {
    match stmt {
        Statement::Query(query) => strip_relation_aliases_in_query(text, query, alias_tracker),
        Statement::CreateTable(create_table) => {
            if let Some(q) = &create_table.query {
                strip_relation_aliases_in_query(text, q, alias_tracker);
            }
        }
        Statement::CreateView(create_view) => {
            strip_relation_aliases_in_query(text, create_view.query.as_ref(), alias_tracker)
        }
        Statement::Insert(insert) => {
            if let Some(query) = &insert.source {
                strip_relation_aliases_in_query(text, query, alias_tracker);
            }
        }
        _ => {}
    }
}

fn strip_relation_aliases_in_query(
    text: &mut String,
    query: &Query,
    alias_tracker: &mut RelationAliasTracker,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            strip_relation_aliases_in_query(text, cte.query.as_ref(), alias_tracker);
        }
    }

    if let SetExpr::Select(select) = query.body.as_ref() {
        strip_relation_aliases_in_select(text, select, alias_tracker);
    }
}

fn strip_relation_aliases_in_select(
    text: &mut String,
    select: &Select,
    alias_tracker: &mut RelationAliasTracker,
) {
    for rel in &select.from {
        strip_relation_aliases_in_table_factor(text, &rel.relation, alias_tracker);
        for join in &rel.joins {
            strip_relation_aliases_in_table_factor(text, &join.relation, alias_tracker);
        }
    }
}

fn strip_relation_aliases_in_table_factor(
    text: &mut String,
    factor: &TableFactor,
    alias_tracker: &mut RelationAliasTracker,
) {
    match factor {
        TableFactor::Derived {
            subquery, alias, ..
        } => {
            strip_relation_aliases_in_query(text, subquery, alias_tracker);
            if let Some(alias) = alias {
                remove_alias_keyword(text, alias_tracker, alias.to_string());
            }
        }
        TableFactor::Table { alias, .. }
        | TableFactor::Function { alias, .. }
        | TableFactor::TableFunction { alias, .. }
        | TableFactor::JsonTable { alias, .. }
        | TableFactor::OpenJsonTable { alias, .. }
        | TableFactor::UNNEST { alias, .. } => {
            if let Some(alias) = alias {
                remove_alias_keyword(text, alias_tracker, alias.to_string());
            }
        }
        TableFactor::NestedJoin {
            table_with_joins,
            alias,
        } => {
            strip_relation_aliases_in_table_with_joins(text, table_with_joins, alias_tracker);
            if let Some(alias) = alias {
                remove_alias_keyword(text, alias_tracker, alias.to_string());
            }
        }
        TableFactor::Pivot { table, alias, .. }
        | TableFactor::Unpivot { table, alias, .. }
        | TableFactor::MatchRecognize { table, alias, .. } => {
            strip_relation_aliases_in_table_factor(text, table, alias_tracker);
            if let Some(alias) = alias {
                remove_alias_keyword(text, alias_tracker, alias.to_string());
            }
        }
        TableFactor::XmlTable { alias, .. } | TableFactor::SemanticView { alias, .. } => {
            if let Some(alias) = alias {
                remove_alias_keyword(text, alias_tracker, alias.to_string());
            }
        }
    }
}

fn strip_relation_aliases_in_table_with_joins(
    text: &mut String,
    rel: &TableWithJoins,
    alias_tracker: &mut RelationAliasTracker,
) {
    strip_relation_aliases_in_table_factor(text, &rel.relation, alias_tracker);
    for join in &rel.joins {
        strip_relation_aliases_in_table_factor(text, &join.relation, alias_tracker);
    }
}

fn remove_alias_keyword(
    text: &mut String,
    alias_tracker: &mut RelationAliasTracker,
    alias_str: String,
) {
    if alias_tracker.next().unwrap_or(true) {
        return;
    }
    let needle = format!(" AS {alias_str}");
    if text.contains(&needle) {
        *text = text.replacen(&needle, &format!(" {alias_str}"), 1);
    }
}

fn hive_format_is_empty(fmt: &HiveFormat) -> bool {
    fmt.row_format.is_none()
        && fmt.storage.is_none()
        && fmt.location.is_none()
        && fmt
            .serde_properties
            .as_ref()
            .is_none_or(|props| props.is_empty())
}

fn format_option_block(keyword: &str, raw: &str, cfg: &FormatterConfig) -> Doc {
    let trimmed = raw.trim();
    let lower = trimmed.to_lowercase();
    let rest = if lower.starts_with(&keyword.to_lowercase()) {
        trimmed[keyword.len()..].trim()
    } else {
        trimmed
    };

    if rest.is_empty() {
        return Doc::Text(apply_keyword_case(keyword, cfg));
    }

    let keyword = apply_keyword_case(keyword, cfg);
    let option_doc = render_option_content(rest, cfg);
    Doc::Group(vec![Doc::Text(keyword), Doc::Space, option_doc])
}

fn doc_has_line(doc: &Doc) -> bool {
    match doc {
        Doc::Line | Doc::SoftLine => true,
        Doc::Indent(inner) => doc_has_line(inner),
        Doc::Group(children) | Doc::Concat(children) => children.iter().any(doc_has_line),
        _ => false,
    }
}

fn dialect_for_kind(kind: &DialectKind) -> Box<dyn Dialect> {
    match kind {
        DialectKind::Ansi => Box::new(AnsiDialect {}),
        DialectKind::Dremio => Box::new(GenericDialect {}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DialectKind, KeywordCase, SelectListStyle};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn format_str(sql: &str, cfg: &FormatterConfig) -> String {
        format_sql(sql, cfg).expect("format")
    }

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
    fn cased_builtins_and_booleans() {
        let cfg = FormatterConfig::default();
        let sql = "select sum(amount), my_func(a), row_number() over(partition by grp order by ts) from t where flag = false";
        let out = format_str(sql, &cfg);
        assert!(
            out.trim()
                == "SELECT SUM(amount), my_func(a), ROW_NUMBER() OVER (\n    PARTITION BY grp\n    ORDER BY ts\n  ) FROM t\nWHERE flag = FALSE",
            "unexpected formatting: {out}"
        );
    }

    #[test]
    fn respects_lower_keyword_case_for_builtins() {
        let cfg = FormatterConfig {
            keyword_case: KeywordCase::Lower,
            ..Default::default()
        };
        let sql = "SELECT MAX(a) FROM t WHERE flag = TRUE";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "select max(a) from t\nwhere flag = true");
    }

    #[test]
    fn formats_inline_select() {
        let cfg = FormatterConfig::default();
        let sql = "SELECT a, b FROM t";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "SELECT a, b FROM t");
    }

    #[test]
    fn formats_raw_sql_when_parse_fails() {
        let cfg = FormatterConfig::default();
        let sql = "unknown_verb select field from thing where flag=true;";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "unknown_verb\nSELECT field\nFROM thing\nWHERE flag = TRUE;"
        );
    }

    #[test]
    fn preserves_trailing_semicolon_for_single_statement() {
        let cfg = FormatterConfig::default();
        let sql = "SELECT a;";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "SELECT a;");
    }

    #[test]
    fn formats_select_breaks_to_per_line_when_overflow() {
        let cfg = FormatterConfig {
            line_length: 15,
            ..Default::default()
        };
        let sql = "SELECT a, b, c, d FROM t";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "SELECT\n  a,\n  b,\n  c,\n  d\nFROM t");
    }

    #[test]
    fn breaks_inline_when_from_would_overflow() {
        let cfg = FormatterConfig {
            line_length: 30,
            ..Default::default()
        };
        let sql = "SELECT a, b FROM very_long_table_name";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "SELECT\n  a,\n  b\nFROM very_long_table_name");
    }

    #[test]
    fn breaks_inline_when_first_join_would_overflow() {
        let cfg = FormatterConfig {
            line_length: 60,
            ..Default::default()
        };
        let sql =
            "SELECT a FROM base INNER JOIN a_very_long_join_relation_name AS j ON j.id = base.id";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "SELECT\n  a\nFROM base\nINNER JOIN a_very_long_join_relation_name AS j\n  ON j.id = base.id"
        );
    }

    #[test]
    fn formats_dremio_path_with_quoted_segments() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = r#"SELECT * FROM Samples."samples.dremio.com"."NYC-taxi-trips""#;
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            r#"SELECT * FROM Samples."samples.dremio.com"."NYC-taxi-trips""#
        );
    }

    #[test]
    fn formats_dremio_ctas_with_partitioning() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = "CREATE TABLE IF NOT EXISTS demoCatalog.test.orders_history PARTITION BY (snapshot_ts) AS SELECT *, CAST(NOW() AS TIMESTAMP) AS snapshot_ts FROM demoCatalog.sales.staging.orders;";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "CREATE TABLE IF NOT EXISTS demoCatalog.test.orders_history\nPARTITION BY (snapshot_ts)\nAS\nSELECT\n  *,\n  CAST(NOW() AS TIMESTAMP) AS snapshot_ts\nFROM demoCatalog.sales.staging.orders;"
        );
    }

    #[test]
    fn formats_dremio_alter_table_reflection() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = r#"
ALTER TABLE demoCatalog.sales.staging."flight_segments"
CREATE RAW REFLECTION reporting
USING DISPLAY (
    flight_id,
    leg_number
)
PARTITION BY (MONTH(created_at));
        "#;
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "ALTER TABLE demoCatalog.sales.staging.\"flight_segments\"\nCREATE RAW REFLECTION reporting\nUSING DISPLAY (\n  flight_id,\n  leg_number\n)\nPARTITION BY (MONTH(created_at));"
        );
    }

    #[test]
    fn formats_dremio_complex_ctas_output() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = r#"CREATE TABLE demoCatalog.reporting."tables".orders_partitioned AS (
SELECT 
    o.id AS order_id,
    CURRENT_TIMESTAMP AS sync_time,
    o.order_number AS order_number,
    co.site_id AS site_id,
    s.brand_id AS brand_id,
    s.country_id AS site_country_id,
    o.created_at AS created_at,
    COALESCE(dist.revenue_share, 0) AS distribution_amount,
    CASE WHEN o.change_time IS NULL THEN 0 ELSE 1 END AS is_changed,
    CASE WHEN o.cancel_time IS NULL THEN 0 ELSE 1 END AS is_canceled
FROM demoCatalog.reporting."tables"."orders" o
JOIN demoCatalog.reporting."tables"."cart_orders" co ON co.order_id = o.id
JOIN demoCatalog.reporting."tables"."order_revenues" orev ON orev.order_id = o.id
JOIN demoCatalog.reporting."tables"."sites" s ON s.id = co.site_id
LEFT JOIN (
    SELECT i.order_id, SUM(i.revenue) AS revenue_share
    FROM demoCatalog.reporting."tables"."order_revenue_items" i
    JOIN demoCatalog.reporting."tables"."product_types" pt ON i.product_type_id = pt.id
    WHERE pt.revenue_group_id = 17
    GROUP BY i.order_id
) dist ON dist.order_id = o.id
WHERE NOT EXISTS (
      SELECT 1
      FROM demoCatalog.reporting."tables"."order_items" oi
      JOIN demoCatalog.reporting."tables"."customer_items" ci ON ci.id = oi.customer_item_id
      WHERE oi.order_id = o.id AND ci.product_type_id = 620
  )
  AND (o.is_test_order = FALSE)
);"#;
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "CREATE TABLE demoCatalog.reporting.\"tables\".orders_partitioned AS\n(\n  SELECT\n    o.id AS order_id,\n    CURRENT_TIMESTAMP AS sync_time,\n    o.order_number AS order_number,\n    co.site_id AS site_id,\n    s.brand_id AS brand_id,\n    s.country_id AS site_country_id,\n    o.created_at AS created_at,\n    COALESCE(dist.revenue_share, 0) AS distribution_amount,\n    CASE WHEN o.change_time IS NULL THEN 0 ELSE 1 END AS is_changed,\n    CASE WHEN o.cancel_time IS NULL THEN 0 ELSE 1 END AS is_canceled\n  FROM demoCatalog.reporting.\"tables\".\"orders\" o\n  INNER JOIN demoCatalog.reporting.\"tables\".\"cart_orders\" co\n    ON co.order_id = o.id\n  INNER JOIN demoCatalog.reporting.\"tables\".\"order_revenues\" orev\n    ON orev.order_id = o.id\n  INNER JOIN demoCatalog.reporting.\"tables\".\"sites\" s\n    ON s.id = co.site_id\n  LEFT JOIN (\n    SELECT\n      i.order_id,\n      SUM(i.revenue) AS revenue_share\n    FROM demoCatalog.reporting.\"tables\".\"order_revenue_items\" i\n    INNER JOIN demoCatalog.reporting.\"tables\".\"product_types\" pt\n      ON i.product_type_id = pt.id\n    WHERE pt.revenue_group_id = 17\n    GROUP BY i.order_id\n  ) dist\n    ON dist.order_id = o.id\n  WHERE NOT EXISTS\n      (\n        SELECT\n          1\n        FROM demoCatalog.reporting.\"tables\".\"order_items\" oi\n        INNER JOIN demoCatalog.reporting.\"tables\".\"customer_items\" ci\n          ON ci.id = oi.customer_item_id\n        WHERE oi.order_id = o.id\n          AND ci.product_type_id = 620\n      )\n    AND (o.is_test_order = FALSE)\n);"
        );
    }

    #[test]
    fn formats_dremio_versioned_clause() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = "SELECT * FROM my_source.my_space.my_table AT BRANCH my_branch AS OF TIMESTAMP '2025-01-01 00:00:00'";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "SELECT *\nFROM my_source.my_space.my_table\nAT BRANCH my_branch\nAS OF TIMESTAMP '2025-01-01 00:00:00'"
        );
    }

    #[test]
    fn passes_through_dremio_use_command() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = "USE Samples.\"samples.dremio.com\";";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "USE Samples.\"samples.dremio.com\";");
    }

    #[test]
    fn passthrough_dremio_branch_commands() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = "create branch my_branch";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "CREATE BRANCH my_branch");
    }

    #[test]
    fn passthrough_dremio_table_maintenance() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = "optimize table my_space.my_table";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "OPTIMIZE TABLE my_space.my_table");
    }

    #[test]
    fn normalizes_dremio_show_branches() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = "show branches in my_catalog";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "SHOW BRANCHES in my_catalog");
    }

    #[test]
    fn formats_dremio_alter_pds() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = "alter pds demo_source.sales.\"chargeback_files\" refresh metadata force update";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "ALTER PDS demo_source.sales.\"chargeback_files\" REFRESH METADATA FORCE UPDATE"
        );
    }

    #[test]
    fn formats_dremio_create_folder() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = "create folder if not exists demoCatalog.sales";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "CREATE FOLDER IF NOT EXISTS demoCatalog.sales");
    }

    #[test]
    fn normalizes_dremio_set_queue() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = "set queue foo";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "SET QUEUE foo");
    }

    #[test]
    fn breaks_long_dremio_command_rest_when_needed() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            line_length: 25,
            ..Default::default()
        };
        let sql = "copy into table my_table using (location 's3://bucket/path/with/long/name')";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "COPY INTO TABLE\n  my_table\n  USING (location 's3://bucket/path/with/long/name')"
        );
    }

    #[test]
    fn breaks_merge_branch_rest() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            line_length: 30,
            ..Default::default()
        };
        let sql = "merge branch feature into main at commit abcdef";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "MERGE BRANCH\n  feature\n  INTO main\n  AT commit abcdef"
        );
    }

    #[test]
    fn breaks_optimize_table_options() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            line_length: 30,
            ..Default::default()
        };
        let sql = "optimize table my_space.my_table with (option = 'x', other = 'y')";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "OPTIMIZE TABLE my_space.my_table\n  WITH (\n    option = 'x',\n    other = 'y'\n  )"
        );
    }

    #[test]
    fn breaks_with_options_per_line_when_long() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            line_length: 40,
            ..Default::default()
        };
        let sql = "optimize table my_space.my_table with (first_option = 'x', second_option = 'y', third_option = 'z')";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "OPTIMIZE TABLE my_space.my_table\n  WITH (\n    first_option = 'x',\n    second_option = 'y',\n    third_option = 'z'\n  )"
        );
    }

    #[test]
    fn formats_pipe_with_options() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            line_length: 30,
            ..Default::default()
        };
        let sql = "create pipe my_pipe with (option = 'x')";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "CREATE PIPE my_pipe\n  WITH (option = 'x')");
    }

    #[test]
    fn formats_acceleration_with_options() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            line_length: 30,
            ..Default::default()
        };
        let sql = "acceleration my_reflection with (refresh = 'auto')";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "ACCELERATION\n  my_reflection\n  WITH (refresh = 'auto')"
        );
    }

    #[test]
    fn breaks_acceleration_using_options() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            line_length: 35,
            ..Default::default()
        };
        let sql = "acceleration my_reflection using (option_one = 'a', option_two = 'b')";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "ACCELERATION\n  my_reflection\n  USING (\n    option_one = 'a',\n    option_two = 'b'\n  )"
        );
    }

    #[test]
    fn formats_manage_acceleration_with_options() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            line_length: 40,
            ..Default::default()
        };
        let sql = "refresh acceleration my_reflection with (refresh = 'auto')";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "REFRESH ACCELERATION\n  my_reflection\n  WITH (refresh = 'auto')"
        );
    }

    #[test]
    fn formats_per_line_select() {
        let cfg = FormatterConfig {
            select_list_style: SelectListStyle::PerLine,
            ..Default::default()
        };
        let sql = "SELECT a, b_long_expr, c FROM t";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "SELECT\n  a,\n  b_long_expr,\n  c\nFROM t");
    }

    #[test]
    fn formats_join_layout() {
        let cfg = FormatterConfig::default();
        let sql =
            "SELECT a FROM foo INNER JOIN bar ON bar.id = foo.id LEFT JOIN baz ON baz.id = foo.id";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "SELECT a FROM foo\nINNER JOIN bar\n  ON bar.id = foo.id\nLEFT JOIN baz\n  ON baz.id = foo.id"
        );
    }

    #[test]
    fn formats_insert_select() {
        let cfg = FormatterConfig::default();
        let sql = "INSERT INTO my_table SELECT a, b FROM source WHERE b > 0 GROUP BY a, b";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "INSERT INTO my_table\nSELECT a, b FROM source\nWHERE b > 0\nGROUP BY\n  a,\n  b"
        );
    }

    #[test]
    fn aligns_join_with_version_clause() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = "SELECT * FROM catalog.orders o AT BRANCH feature_x AS OF TIMESTAMP '2024-10-01 12:00:00' INNER JOIN catalog.order_items i ON i.order_id = o.order_id AND i.valid_from <= o.order_ts";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "SELECT *\nFROM catalog.orders o\nAT BRANCH feature_x\nAS OF TIMESTAMP '2024-10-01 12:00:00'\nINNER JOIN catalog.order_items i\n  ON i.order_id = o.order_id\n  AND i.valid_from <= o.order_ts"
        );
    }

    #[test]
    fn formats_joined_subquery_as_select_block() {
        let cfg = FormatterConfig::default();
        let sql = "SELECT * FROM base LEFT JOIN (SELECT customer_id, max(order_ts) AS last_order_ts FROM fact_orders GROUP BY customer_id) last_o ON last_o.customer_id = base.customer_id";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "SELECT *\nFROM base\nLEFT JOIN (\n  SELECT\n    customer_id,\n    MAX(order_ts) AS last_order_ts\n  FROM fact_orders\n  GROUP BY customer_id\n) last_o\n  ON last_o.customer_id = base.customer_id"
        );
    }

    #[test]
    fn formats_group_having_order_limit_offset() {
        let cfg = FormatterConfig::default();
        let sql = "SELECT a, b FROM t WHERE a > 1 GROUP BY a HAVING SUM(b) > 0 ORDER BY a DESC, b LIMIT 10 OFFSET 5";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "SELECT a, b FROM t\nWHERE a > 1\nGROUP BY a\nHAVING SUM(b) > 0\nORDER BY a DESC, b\nLIMIT 10\nOFFSET 5"
        );
    }

    #[test]
    fn formats_distinct_per_line() {
        let cfg = FormatterConfig {
            select_list_style: SelectListStyle::PerLine,
            ..Default::default()
        };
        let sql = "SELECT DISTINCT a, b, c, d, e, f FROM t";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "SELECT DISTINCT\n  a,\n  b,\n  c,\n  d,\n  e,\n  f\nFROM t"
        );
    }

    #[test]
    fn formats_multiple_statements_separately() {
        let cfg = FormatterConfig::default();
        let sql = "SELECT a;\nSELECT b, c FROM t;";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "SELECT a;\n\nSELECT b, c FROM t;");
    }

    #[test]
    fn formats_multiple_dremio_statements_with_semicolons() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = "USE my_space;\nSELECT a FROM my_space.table_one;";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "USE my_space;\n\nSELECT a FROM my_space.table_one;"
        );
    }

    #[test]
    fn formats_cte() {
        let cfg = FormatterConfig::default();
        let sql = r#"
        WITH cte AS (
            SELECT a, b FROM t
        )
        SELECT a FROM cte"#;
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "WITH cte AS (\n  SELECT a, b FROM t\n)\n\nSELECT a FROM cte"
        );
    }

    #[test]
    fn breaks_boolean_conditions() {
        let cfg = FormatterConfig {
            line_length: 30,
            ..Default::default()
        };
        let sql = "SELECT a FROM t WHERE a > 1 AND b > 2 AND c > 3";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "SELECT a FROM t\nWHERE a > 1\n  AND b > 2\n  AND c > 3"
        );
    }

    #[test]
    fn breaks_group_by_and_order_by_lists() {
        let cfg = FormatterConfig {
            line_length: 25,
            ..Default::default()
        };
        let sql = "SELECT a FROM t GROUP BY a, very_long_group, another_long_group ORDER BY a, very_long_order, another_long_order";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "SELECT a FROM t\nGROUP BY\n  a,\n  very_long_group,\n  another_long_group\nORDER BY a,\n  very_long_order,\n  another_long_order"
        );
    }

    #[test]
    fn breaks_on_clause_conditions() {
        let cfg = FormatterConfig {
            line_length: 30,
            ..Default::default()
        };
        let sql = "SELECT * FROM t JOIN u ON t.id = u.id AND t.tenant = u.tenant";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "SELECT *\nFROM t\nINNER JOIN u\n  ON t.id = u.id\n  AND t.tenant = u.tenant"
        );
    }

    #[test]
    fn breaks_nested_boolean_with_parens() {
        let cfg = FormatterConfig {
            line_length: 35,
            ..Default::default()
        };
        let sql = "SELECT * FROM t WHERE (a = 1 AND b = 2) OR (c = 3 AND (d = 4 OR e = 5))";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "SELECT * FROM t\nWHERE (\n    a = 1\n    AND b = 2\n  )\n  OR (\n    c = 3\n    AND (\n      d = 4\n      OR e = 5\n    )\n  )"
        );
    }

    #[test]
    fn formats_case_expression() {
        let cfg = FormatterConfig {
            line_length: 40,
            ..Default::default()
        };
        let sql = "SELECT CASE WHEN a = 1 THEN 'one' WHEN a = 2 THEN 'two' ELSE 'other' END AS label FROM t";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "SELECT\n  CASE\n    WHEN a = 1 THEN 'one'\n    WHEN a = 2 THEN 'two'\n    ELSE 'other'\n  END AS label\nFROM t"
        );
    }

    #[test]
    fn formats_long_single_when_case_multiline() {
        let cfg = FormatterConfig::default();
        let sql = "SELECT CASE WHEN o.status IN ('CANCELLED', 'RETURNED') THEN 1 ELSE 0 END AS is_problem FROM orders AS o";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "SELECT\n  CASE\n    WHEN o.status IN ('CANCELLED', 'RETURNED') THEN 1\n    ELSE 0\n  END AS is_problem\nFROM orders AS o"
        );
    }

    #[test]
    fn formats_multiline_window_spec() {
        let cfg = FormatterConfig::default();
        let sql =
            "SELECT ROW_NUMBER() OVER (PARTITION BY customer_id ORDER BY order_ts) AS order_seq FROM fact_orders";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "SELECT ROW_NUMBER() OVER (\n    PARTITION BY customer_id\n    ORDER BY order_ts\n  ) AS order_seq FROM fact_orders"
        );
    }

    #[test]
    fn keeps_simple_window_inline() {
        let cfg = FormatterConfig::default();
        let sql = "SELECT SUM(x) OVER (PARTITION BY grp) FROM t";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "SELECT SUM(x) OVER (PARTITION BY grp) FROM t");
    }

    #[test]
    fn formats_nested_case_branches() {
        let cfg = FormatterConfig::default();
        let sql = "SELECT CASE WHEN lag(event_ts) OVER (PARTITION BY user_id ORDER BY event_ts) IS NULL THEN 1 WHEN event_ts - lag(event_ts) OVER (PARTITION BY user_id ORDER BY event_ts) > INTERVAL '30' MINUTE THEN 1 ELSE 0 END AS is_session_start FROM events";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "SELECT\n  CASE\n    WHEN LAG(event_ts) OVER (\n      PARTITION BY user_id\n      ORDER BY event_ts\n    ) IS NULL THEN 1\n    WHEN event_ts - LAG(event_ts) OVER (\n      PARTITION BY user_id\n      ORDER BY event_ts\n    ) > INTERVAL '30' MINUTE THEN 1\n    ELSE 0\n  END AS is_session_start\nFROM events"
        );
    }

    #[test]
    fn formats_subquery_expression() {
        let cfg = FormatterConfig {
            line_length: 80,
            ..Default::default()
        };
        let sql = "SELECT (SELECT max(x) FROM other) AS m FROM t";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "SELECT (SELECT MAX(x) FROM other) AS m FROM t");
    }

    #[test]
    fn formats_create_table_as_select() {
        let cfg = FormatterConfig::default();
        let sql =
            "CREATE TABLE my_table AS SELECT id, value FROM source WHERE value > 0 ORDER BY id";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "CREATE TABLE my_table AS\nSELECT id, value FROM source\nWHERE value > 0\nORDER BY id"
        );
    }

    #[test]
    fn formats_reflection_with_options_multiline() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = "alter reflection analytics_space.daily_revenue_by_country_reflection using display name 'Daily revenue by country reflection' partition by (order_date) distribute by (country) sort by (order_date, country)";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "ALTER REFLECTION\n  analytics_space.daily_revenue_by_country_reflection\n  USING\n    DISPLAY NAME 'Daily revenue by country reflection'\n    PARTITION BY (order_date)\n    DISTRIBUTE BY (country)\n    SORT BY (order_date, country)"
        );
    }

    #[test]
    fn formats_dremio_reflection_command() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            line_length: 40,
            ..Default::default()
        };
        let sql = "create reflection my_reflection using table foo";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "CREATE REFLECTION\n  my_reflection\n  USING\n    TABLE foo"
        );
    }

    #[test]
    fn formats_dremio_show_reflections() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = "show reflections in my_space";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "SHOW REFLECTIONS in my_space");
    }

    #[test]
    fn formats_dremio_refresh_reflection() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = "refresh reflection foo";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "REFRESH REFLECTION foo");
    }

    #[test]
    fn applies_keyword_case_lower() {
        let cfg = FormatterConfig {
            keyword_case: KeywordCase::Lower,
            ..Default::default()
        };
        let sql = "SELECT a, b FROM t WHERE a > 1";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "select a, b from t\nwhere a > 1");
    }

    #[test]
    fn applies_keyword_case_capitalize() {
        let cfg = FormatterConfig {
            keyword_case: KeywordCase::Capitalize,
            ..Default::default()
        };
        let sql = "SELECT a, b FROM t";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "Select a, b From t");
    }

    #[test]
    fn formats_create_table_basic() {
        let cfg = FormatterConfig::default();
        let sql =
            "CREATE TABLE IF NOT EXISTS my_table (id INT NOT NULL, name TEXT, PRIMARY KEY (id))";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "CREATE TABLE IF NOT EXISTS my_table (\n  id INT NOT NULL,\n  name TEXT,\n  PRIMARY KEY (id)\n)"
        );
    }

    #[test]
    fn formats_create_view_basic() {
        let cfg = FormatterConfig::default();
        let sql = "CREATE VIEW my_view AS SELECT a, b FROM t";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), "CREATE VIEW my_view AS\nSELECT a, b FROM t");
    }

    #[test]
    fn formats_create_or_replace_view_inline_as() {
        let cfg = FormatterConfig::default();
        let sql =
            "CREATE OR REPLACE VIEW demoCatalog.sales.staging.geo.continent AS SELECT * FROM source_cluster.geo.\"continent\"";
        let out = format_str(sql, &cfg);
        assert!(out.trim().starts_with(
            "CREATE OR REPLACE VIEW demoCatalog.sales.staging.geo.continent AS\nSELECT"
        ));
    }

    #[test]
    fn formats_dremio_view_with_long_path_and_functions() {
        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };
        let sql = "\
CREATE OR REPLACE VIEW demoCatalog.sales.staging.analytics.order_customers AS
SELECT
  o.id AS order_id,
  CURRENT_TIMESTAMP AS sync_time,
  o.site_id AS site_id,
  o.site_country_id AS site_country_id,
  o.created_at AS order_created_at,
  COALESCE(MD5(c.email), '') AS email_hash
FROM demoCatalog.sales.staging.analytics.orders o
JOIN demoCatalog.sales.staging.crm.customers c
  ON c.order_id = o.id;
";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "\
CREATE OR REPLACE VIEW demoCatalog.sales.staging.analytics.order_customers AS
SELECT
  o.id AS order_id,
  CURRENT_TIMESTAMP AS sync_time,
  o.site_id AS site_id,
  o.site_country_id AS site_country_id,
  o.created_at AS order_created_at,
  COALESCE(MD5(c.email), '') AS email_hash
FROM demoCatalog.sales.staging.analytics.orders o
INNER JOIN demoCatalog.sales.staging.crm.customers c
  ON c.order_id = o.id;"
        );
    }

    #[test]
    fn formats_sql_with_jinja_blocks() {
        let cfg = FormatterConfig::default();
        let sql = "\
SELECT
  *
FROM {{ ref(\"orders_table\") }}
WHERE order_date >= {{ start_date }}
{% if include_cancelled %}
AND status <> 'CANCELLED'
{% endif %}
;";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "\
SELECT
  *
FROM {{ ref(\"orders_table\") }}
WHERE order_date >= {{ start_date }}
{% if include_cancelled %}
AND status <> 'CANCELLED'
{% endif %}
;"
        );
    }

    #[test]
    fn preserves_simple_comments_without_failure() {
        let cfg = FormatterConfig::default();
        let sql = "\
-- Only include active customers
SELECT id, full_name FROM dim_customers;
/* block comment */
SELECT country_code FROM dim_countries;
";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "\
-- Only include active customers
SELECT id, full_name FROM dim_customers;

/* block comment */
SELECT country_code FROM dim_countries;"
        );
    }

    #[test]
    fn avoids_trailing_space_before_newline_after_inline_comment() {
        let cfg = FormatterConfig::default();
        let sql = "SELECT 1; -- note\nSELECT 2;";
        let out = format_str(sql, &cfg);
        assert!(
            !out.contains("-- note \n"),
            "unexpected trailing space before newline:\n{out}"
        );
    }

    #[test]
    fn formats_insert_values_with_leading_comment() {
        let cfg = FormatterConfig::default();
        let sql = "\
CREATE TABLE IF NOT EXISTS demoCatalog.sandbox.orders_demo (
    id INT,
    user_id INT,
    created_at TIMESTAMP,
    total_amount DECIMAL(10, 2)
);

-- Use CURRENT_DATE so \"last 30 days\" is relative to runtime.
INSERT INTO demoCatalog.sandbox.orders_demo VALUES
(101, 1, CURRENT_DATE - INTERVAL '10' DAY, 150.75),
(102, 1, CURRENT_DATE - INTERVAL '45' DAY, 200.00),
(103, 2, CURRENT_DATE - INTERVAL '5' DAY, 75.50),
(104, 3, CURRENT_DATE - INTERVAL '25' DAY, 310.20),
(105, 3, CURRENT_DATE - INTERVAL '90' DAY, 99.99);
";
        let out = format_str(sql, &cfg);
        assert_eq!(
            out.trim(),
            "\
CREATE TABLE IF NOT EXISTS demoCatalog.sandbox.orders_demo (
  id INT,
  user_id INT,
  created_at TIMESTAMP,
  total_amount DECIMAL(10,2)
);

-- Use CURRENT_DATE so \"last 30 days\" is relative to runtime.
INSERT INTO demoCatalog.sandbox.orders_demo
VALUES
  (101, 1, CURRENT_DATE - INTERVAL '10' DAY, 150.75),
  (102, 1, CURRENT_DATE - INTERVAL '45' DAY, 200.00),
  (103, 2, CURRENT_DATE - INTERVAL '5' DAY, 75.50),
  (104, 3, CURRENT_DATE - INTERVAL '25' DAY, 310.20),
  (105, 3, CURRENT_DATE - INTERVAL '90' DAY, 99.99);"
        );
    }

    #[test]
    fn formats_drop_create_with_jinja_identifiers_and_hint() {
        let cfg = FormatterConfig::default();
        let sql = "\
DROP TABLE IF EXISTS demoCatalog.app.staging.reports.{{ ti.xcom_pull(task_ids='determine_target_table') }};

CREATE TABLE demoCatalog.app.staging.reports.{{ ti.xcom_pull(task_ids='determine_target_table') }} AS
SELECT /*+ no_reflections */
    *
FROM external_cluster.app.raw_segments;
";
        let out = format_str(sql, &cfg);
        assert_eq!(out.trim(), sql.trim());
    }

    #[test]
    fn formats_all_dremio_reference_command_fixtures_idempotently() {
        let files = reference_command_fixture_paths();
        assert_eq!(files.len(), 57, "expected 57 reference command fixtures");

        let cfg = FormatterConfig {
            dialect: DialectKind::Dremio,
            ..Default::default()
        };

        for path in files {
            let sql = fs::read_to_string(&path).expect("read fixture");
            let once = format_sql(&sql, &cfg).unwrap_or_else(|err| {
                panic!("format failed for {:?}: {err}", path);
            });
            let twice = format_sql(&once, &cfg).unwrap_or_else(|err| {
                panic!("reformat failed for {:?}: {err}", path);
            });
            assert_eq!(once, twice, "format not idempotent for {:?}", path);
        }
    }

    #[test]
    fn formats_all_dremio_reference_command_fixtures_for_upper_and_lower_keyword_case() {
        let files = reference_command_fixture_paths();
        for keyword_case in [KeywordCase::Upper, KeywordCase::Lower] {
            let cfg = FormatterConfig {
                dialect: DialectKind::Dremio,
                keyword_case,
                ..Default::default()
            };
            for path in &files {
                let sql = fs::read_to_string(path).expect("read fixture");
                format_sql(&sql, &cfg).unwrap_or_else(|err| {
                    panic!(
                        "format failed for {:?} with keyword_case {:?}: {err}",
                        path, keyword_case
                    )
                });
            }
        }
    }
}
