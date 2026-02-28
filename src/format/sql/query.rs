use anyhow::Result;
use sqlparser::ast::{
    Cte, GroupByExpr, LimitClause, OrderByKind, Query, Select, SelectFlavor, SelectItem, SetExpr,
    Values, With,
};

use crate::config::FormatterConfig;
use crate::format::doc::Doc;
use crate::parser::DremioVersionClause;

pub(super) fn format_query(
    query: &Query,
    cfg: &FormatterConfig,
    version: Option<DremioVersionClause>,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Result<Doc> {
    format_query_with_layout_preference(query, cfg, version, alias_tracker, false)
}

pub(super) fn format_query_with_layout_preference(
    query: &Query,
    cfg: &FormatterConfig,
    version: Option<DremioVersionClause>,
    alias_tracker: &mut super::RelationAliasTracker,
    prefer_multiline: bool,
) -> Result<Doc> {
    if query_needs_safe_fallback(query) {
        return Ok(Doc::Text(query.to_string()));
    }

    let mut parts = Vec::new();

    if let Some(with) = &query.with {
        parts.push(format_with(with, cfg, alias_tracker)?);
        parts.push(Doc::Line);
        parts.push(Doc::Line);
    }

    match query.body.as_ref() {
        SetExpr::Select(s) => {
            let select = s.as_ref();

            let mut layout = super::choose_layout(select, cfg);
            if prefer_multiline && matches!(layout, super::SelectLayout::Inline) {
                layout = super::SelectLayout::PerLine;
            }
            let select_doc = super::format_select(select, layout, cfg, alias_tracker);
            parts.push(select_doc);

            let is_single_wildcard = select.projection.len() == 1
                && matches!(
                    select.projection[0],
                    SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard { .. }
                );

            if let Some(from_doc) = super::format_from(&select.from, cfg, version, alias_tracker) {
                let needs_newline = super::doc_has_line(&from_doc);
                match layout {
                    super::SelectLayout::Inline if needs_newline && is_single_wildcard => {
                        parts.push(Doc::Line);
                        parts.push(from_doc);
                    }
                    super::SelectLayout::Inline => {
                        parts.push(Doc::Space);
                        parts.push(from_doc);
                    }
                    _ => {
                        parts.push(Doc::Line);
                        parts.push(from_doc);
                    }
                }
            }

            if let Some(selection) = &select.selection {
                parts.push(Doc::Line);
                parts.push(super::format_boolean_clause(
                    "WHERE",
                    selection,
                    cfg,
                    alias_tracker,
                ));
            }

            match &select.group_by {
                GroupByExpr::All(modifiers) => {
                    parts.push(Doc::Line);
                    if modifiers.is_empty() {
                        parts.push(Doc::Text("GROUP BY ALL".into()));
                    } else {
                        parts.push(Doc::Text(select.group_by.to_string()));
                    }
                }
                GroupByExpr::Expressions(exprs, modifiers) if !exprs.is_empty() => {
                    parts.push(Doc::Line);
                    if modifiers.is_empty() {
                        let exprs = exprs.iter().map(|e| e.to_string()).collect::<Vec<_>>();
                        if exprs.len() > 1 {
                            parts.push(super::format_comma_clause_per_line("GROUP BY", exprs, cfg));
                        } else {
                            parts.push(super::format_comma_clause("GROUP BY", exprs, cfg));
                        }
                    } else {
                        parts.push(Doc::Text(select.group_by.to_string()));
                    }
                }
                _ => {}
            }

            if let Some(having) = &select.having {
                parts.push(Doc::Line);
                parts.push(super::format_boolean_clause(
                    "HAVING",
                    having,
                    cfg,
                    alias_tracker,
                ));
            }

            if !select.cluster_by.is_empty() {
                let items = select
                    .cluster_by
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>();
                parts.push(Doc::Line);
                parts.push(super::format_comma_clause("CLUSTER BY", items, cfg));
            }

            if !select.distribute_by.is_empty() {
                let items = select
                    .distribute_by
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>();
                parts.push(Doc::Line);
                parts.push(super::format_comma_clause("DISTRIBUTE BY", items, cfg));
            }

            if !select.sort_by.is_empty() {
                let items = select
                    .sort_by
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>();
                parts.push(Doc::Line);
                parts.push(super::format_comma_clause("SORT BY", items, cfg));
            }

            if select.window_before_qualify {
                if !select.named_window.is_empty() {
                    parts.push(Doc::Line);
                    parts.push(format_window_clause(&select.named_window, cfg));
                }
                if let Some(qualify) = &select.qualify {
                    parts.push(Doc::Line);
                    parts.push(super::format_boolean_clause(
                        "QUALIFY",
                        qualify,
                        cfg,
                        alias_tracker,
                    ));
                }
            } else {
                if let Some(qualify) = &select.qualify {
                    parts.push(Doc::Line);
                    parts.push(super::format_boolean_clause(
                        "QUALIFY",
                        qualify,
                        cfg,
                        alias_tracker,
                    ));
                }
                if !select.named_window.is_empty() {
                    parts.push(Doc::Line);
                    parts.push(format_window_clause(&select.named_window, cfg));
                }
            }

            append_query_tail(&mut parts, query, cfg);
            Ok(Doc::Group(parts))
        }
        SetExpr::Query(nested) => {
            let inner = format_query_with_layout_preference(
                nested.as_ref(),
                cfg,
                version,
                alias_tracker,
                prefer_multiline,
            )?;
            parts.push(Doc::Group(vec![
                Doc::Text("(".into()),
                Doc::Line,
                Doc::Indent(Box::new(inner)),
                Doc::Line,
                Doc::Text(")".into()),
            ]));
            append_query_tail(&mut parts, query, cfg);
            Ok(Doc::Group(parts))
        }
        SetExpr::Values(values) => {
            parts.push(format_values(values, cfg));
            append_query_tail(&mut parts, query, cfg);
            Ok(Doc::Group(parts))
        }
        other => {
            parts.push(Doc::Text(other.to_string()));
            append_query_tail(&mut parts, query, cfg);
            Ok(Doc::Group(parts))
        }
    }
}

fn append_query_tail(parts: &mut Vec<Doc>, query: &Query, cfg: &FormatterConfig) {
    if let Some(order_by) = &query.order_by {
        parts.push(Doc::Line);
        if order_by.interpolate.is_none() {
            match &order_by.kind {
                OrderByKind::Expressions(exprs) => {
                    let order: Vec<String> = exprs.iter().map(|o| o.to_string()).collect();
                    parts.push(super::format_comma_clause("ORDER BY", order, cfg));
                }
                OrderByKind::All(_) => parts.push(Doc::Text(order_by.to_string())),
            }
        } else {
            parts.push(Doc::Text(order_by.to_string()));
        }
    }

    if let Some(limit_clause) = &query.limit_clause {
        match limit_clause {
            LimitClause::LimitOffset {
                limit,
                offset,
                limit_by,
            } => {
                if let Some(limit) = limit {
                    parts.push(Doc::Line);
                    parts.push(Doc::Text(format!(
                        "{} {}",
                        super::apply_keyword_case("LIMIT", cfg),
                        limit
                    )));
                }

                if let Some(offset) = offset {
                    parts.push(Doc::Line);
                    let offset_str = offset.to_string();
                    let rendered = offset_str
                        .strip_prefix("OFFSET ")
                        .map(|rest| format!("{} {rest}", super::apply_keyword_case("OFFSET", cfg)))
                        .unwrap_or_else(|| {
                            format!("{} {offset_str}", super::apply_keyword_case("OFFSET", cfg))
                        });
                    parts.push(Doc::Text(rendered));
                }

                if !limit_by.is_empty() {
                    let items = limit_by
                        .iter()
                        .map(|expr| expr.to_string())
                        .collect::<Vec<_>>();
                    parts.push(Doc::Line);
                    parts.push(super::format_comma_clause("BY", items, cfg));
                }
            }
            LimitClause::OffsetCommaLimit { .. } => {
                parts.push(Doc::Line);
                parts.push(Doc::Text(limit_clause.to_string().trim().to_string()));
            }
        }
    }

    if let Some(fetch) = &query.fetch {
        parts.push(Doc::Line);
        parts.push(Doc::Text(fetch.to_string()));
    }

    if !query.locks.is_empty() {
        parts.push(Doc::Line);
        let lock_text = query
            .locks
            .iter()
            .map(|lock| lock.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        parts.push(Doc::Text(lock_text));
    }

    if let Some(for_clause) = &query.for_clause {
        parts.push(Doc::Line);
        parts.push(Doc::Text(for_clause.to_string()));
    }
}

fn format_values(values: &Values, cfg: &FormatterConfig) -> Doc {
    let rows: Vec<Doc> = values
        .rows
        .iter()
        .map(|row| {
            let exprs: Vec<String> = row.iter().map(|e| e.to_string()).collect();
            let paren = super::format_parenthesized_inline(exprs);
            if values.explicit_row {
                Doc::Group(vec![super::keyword_doc(cfg, "ROW"), Doc::Space, paren])
            } else {
                paren
            }
        })
        .collect();

    let mut parts = vec![super::keyword_doc(cfg, "VALUES")];
    if rows.len() == 1 {
        parts.push(Doc::Space);
        parts.push(rows[0].clone());
    } else {
        let mut row_parts = Vec::new();
        for (idx, row) in rows.iter().enumerate() {
            row_parts.push(row.clone());
            if idx + 1 < rows.len() {
                row_parts.push(Doc::Text(",".into()));
                row_parts.push(Doc::Line);
            }
        }
        parts.push(Doc::Line);
        parts.push(Doc::Indent(Box::new(Doc::Group(row_parts))));
    }

    Doc::Group(parts)
}

fn format_with(
    with: &With,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Result<Doc> {
    let mut parts = Vec::new();
    parts.push(super::keyword_doc(cfg, "WITH"));
    if with.recursive {
        parts.push(Doc::Space);
        parts.push(super::keyword_doc(cfg, "RECURSIVE"));
    }

    for (idx, cte) in with.cte_tables.iter().enumerate() {
        if idx == 0 {
            parts.push(Doc::Space);
        } else {
            parts.push(Doc::Text(",".into()));
            parts.push(Doc::Line);
        }
        parts.push(format_cte(cte, cfg, alias_tracker)?);
    }

    Ok(Doc::Group(parts))
}

fn format_cte(
    cte: &Cte,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Result<Doc> {
    let mut head_parts = Vec::new();
    head_parts.push(Doc::Text(cte.alias.to_string()));
    head_parts.push(Doc::Space);
    head_parts.push(super::keyword_doc(cfg, "AS"));
    if let Some(mat) = &cte.materialized {
        head_parts.push(Doc::Space);
        head_parts.push(Doc::Text(mat.to_string()));
    }
    head_parts.push(Doc::Space);
    head_parts.push(Doc::Text("(".into()));

    let mut parts = Vec::new();
    parts.push(Doc::Group(head_parts));
    parts.push(Doc::Line);
    let body = format_query(cte.query.as_ref(), cfg, None, alias_tracker)?;
    parts.push(Doc::Indent(Box::new(body)));
    parts.push(Doc::Line);
    parts.push(Doc::Text(")".into()));

    if let Some(from) = &cte.from {
        parts.push(Doc::Space);
        parts.push(super::keyword_doc(cfg, "FROM"));
        parts.push(Doc::Space);
        parts.push(Doc::Text(from.to_string()));
    }

    Ok(Doc::Group(parts))
}

fn format_window_clause(
    windows: &[sqlparser::ast::NamedWindowDefinition],
    cfg: &FormatterConfig,
) -> Doc {
    let items = windows
        .iter()
        .map(|window| window.to_string())
        .collect::<Vec<_>>();
    super::format_comma_clause("WINDOW", items, cfg)
}

fn query_needs_safe_fallback(query: &Query) -> bool {
    if query.settings.is_some() || query.format_clause.is_some() || !query.pipe_operators.is_empty()
    {
        return true;
    }

    match query.body.as_ref() {
        SetExpr::Select(select) => select_needs_safe_fallback(select),
        SetExpr::Query(inner) => query_needs_safe_fallback(inner),
        _ => false,
    }
}

fn select_needs_safe_fallback(select: &Select) -> bool {
    select.top.is_some()
        || select.optimizer_hint.is_some()
        || select.select_modifiers.is_some()
        || select.exclude.is_some()
        || select.into.is_some()
        || !select.lateral_views.is_empty()
        || select.prewhere.is_some()
        || select.value_table_mode.is_some()
        || !select.connect_by.is_empty()
        || !matches!(select.flavor, SelectFlavor::Standard)
}
