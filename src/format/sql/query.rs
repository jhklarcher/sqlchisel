use anyhow::Result;
use sqlparser::ast::{Cte, GroupByExpr, Query, SelectItem, SetExpr, Values, With};

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
                GroupByExpr::All => {
                    parts.push(Doc::Line);
                    parts.push(Doc::Text("GROUP BY ALL".into()));
                }
                GroupByExpr::Expressions(exprs) if !exprs.is_empty() => {
                    let exprs = exprs.iter().map(|e| e.to_string()).collect::<Vec<_>>();
                    parts.push(Doc::Line);
                    if exprs.len() > 1 {
                        parts.push(super::format_comma_clause_per_line("GROUP BY", exprs, cfg));
                    } else {
                        parts.push(super::format_comma_clause("GROUP BY", exprs, cfg));
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
    if !query.order_by.is_empty() {
        let order: Vec<String> = query.order_by.iter().map(|o| o.to_string()).collect();
        parts.push(Doc::Line);
        parts.push(super::format_comma_clause("ORDER BY", order, cfg));
    }

    if let Some(limit) = &query.limit {
        parts.push(Doc::Line);
        parts.push(Doc::Text(format!(
            "{} {}",
            super::apply_keyword_case("LIMIT", cfg),
            limit
        )));
    }

    if let Some(offset) = &query.offset {
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
