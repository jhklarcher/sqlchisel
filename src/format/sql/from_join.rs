use sqlparser::ast::{
    Expr, Function, Join, JoinOperator, ObjectNamePart, TableFactor, TableWithJoins,
};

use crate::config::FormatterConfig;
use crate::format::doc::Doc;
use crate::parser::DremioVersionClause;

pub(super) fn format_from(
    from: &[TableWithJoins],
    cfg: &FormatterConfig,
    version: Option<DremioVersionClause>,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Option<Doc> {
    if from.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    for (idx, rel) in from.iter().enumerate() {
        let rel_doc = format_table_factor(&rel.relation, cfg, alias_tracker);
        let needs_from_indent = !matches!(rel.relation, TableFactor::TableFunction { .. });
        if idx == 0 {
            let rel_doc = if needs_from_indent {
                Doc::Indent(Box::new(rel_doc))
            } else {
                rel_doc
            };
            parts.push(Doc::Group(vec![
                Doc::Text(super::apply_keyword_case("FROM", cfg)),
                Doc::Space,
                rel_doc,
            ]));
            if let Some(v) = &version {
                parts.push(Doc::Line);
                parts.push(super::format_dremio_version_clause(v, cfg));
            }
        } else {
            parts.push(Doc::Line);
            parts.push(rel_doc);
        }

        if !rel.joins.is_empty() {
            for join in &rel.joins {
                parts.push(Doc::Line);
                parts.push(format_join(join, cfg, alias_tracker));
            }
        }
    }

    Some(Doc::Group(parts))
}

fn format_table_factor(
    factor: &TableFactor,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Doc {
    match factor {
        TableFactor::TableFunction { expr, alias } => {
            let mut parts = Vec::new();
            parts.push(super::keyword_doc(cfg, "TABLE"));
            parts.push(Doc::Space);
            parts.push(Doc::Text("(".into()));
            let inner = format_table_function_expr(expr, cfg, alias_tracker);
            parts.push(Doc::Line);
            parts.push(Doc::Indent(Box::new(inner)));
            parts.push(Doc::Line);
            parts.push(Doc::Text(")".into()));
            if let Some(alias) = alias {
                let has_as = alias_tracker.next().unwrap_or(true);
                parts.push(Doc::Space);
                if has_as {
                    parts.push(super::keyword_doc(cfg, "AS"));
                    parts.push(Doc::Space);
                }
                parts.push(Doc::Text(alias.to_string()));
            }
            Doc::Group(parts)
        }
        TableFactor::Derived {
            lateral,
            subquery,
            alias,
            ..
        } => {
            let mut parts = Vec::new();
            if *lateral {
                parts.push(super::keyword_doc(cfg, "LATERAL"));
                parts.push(Doc::Space);
            }
            parts.push(Doc::Text("(".into()));
            let inner = super::format_query_with_layout_preference(
                subquery,
                cfg,
                None,
                alias_tracker,
                true,
            )
            .unwrap_or_else(|_| Doc::Text(subquery.to_string()));
            parts.push(Doc::Line);
            parts.push(Doc::Indent(Box::new(inner)));
            parts.push(Doc::Line);
            parts.push(Doc::Text(")".into()));
            if let Some(alias) = alias {
                let has_as = alias_tracker.next().unwrap_or(true);
                parts.push(Doc::Space);
                if has_as {
                    parts.push(super::keyword_doc(cfg, "AS"));
                    parts.push(Doc::Space);
                }
                parts.push(Doc::Text(alias.to_string()));
            }
            Doc::Group(parts)
        }
        other => {
            let mut text = other.to_string();
            if let Some(alias) = table_factor_alias_str(other) {
                let has_as = alias_tracker.next().unwrap_or(true);
                if !has_as {
                    let needle = format!(" AS {alias}");
                    if let Some(pos) = text.find(&needle) {
                        text.replace_range(pos..pos + needle.len(), &format!(" {alias}"));
                    }
                }
            }
            Doc::Text(text)
        }
    }
}

fn format_table_function_expr(
    expr: &Expr,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Doc {
    if let Expr::Function(func) = expr {
        return format_function_invocation(func, cfg, alias_tracker);
    }
    super::format_expr(expr, (cfg.line_length / 2).max(1), cfg, alias_tracker)
}

fn format_function_invocation(
    func: &Function,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Doc {
    let mut name = func.name.clone();
    if name.0.len() == 1 {
        if let Some(ObjectNamePart::Identifier(ident)) = name.0.first_mut() {
            if ident.quote_style.is_none() {
                ident.value = super::apply_keyword_case(&ident.value, cfg);
            }
        }
    }

    let head = vec![Doc::Text(name.to_string()), Doc::Text("(".into())];
    let tail = vec![Doc::Text(")".into())];

    let body = match &func.args {
        sqlparser::ast::FunctionArguments::None => None,
        sqlparser::ast::FunctionArguments::Subquery(q) => {
            let formatted = super::format_query(q, cfg, None, alias_tracker)
                .unwrap_or_else(|_| Doc::Text(q.to_string()));
            Some(Doc::Group(vec![
                Doc::Line,
                Doc::Indent(Box::new(formatted)),
                Doc::Line,
            ]))
        }
        sqlparser::ast::FunctionArguments::List(list) => {
            let mut arg_parts = Vec::new();
            if let Some(dup) = list.duplicate_treatment {
                arg_parts.push(Doc::Text(super::apply_keyword_case(&dup.to_string(), cfg)));
                if !list.args.is_empty() {
                    arg_parts.push(Doc::Space);
                }
            }
            for (idx, arg) in list.args.iter().enumerate() {
                arg_parts.push(Doc::Text(arg.to_string()));
                if idx + 1 < list.args.len() {
                    arg_parts.push(Doc::Text(",".into()));
                    arg_parts.push(Doc::Line);
                }
            }
            if !list.clauses.is_empty() {
                if !arg_parts.is_empty() {
                    arg_parts.push(Doc::Line);
                }
                let clause_text: Vec<String> = list.clauses.iter().map(|c| c.to_string()).collect();
                arg_parts.push(Doc::Text(clause_text.join(" ")));
            }
            Some(Doc::Group(vec![
                Doc::Line,
                Doc::Indent(Box::new(Doc::Group(arg_parts))),
                Doc::Line,
            ]))
        }
    };

    let mut parts = Vec::new();
    parts.push(Doc::Group(head));
    if let Some(body) = body {
        parts.push(body);
    }
    parts.push(Doc::Group(tail));

    if let Some(window) = &func.over {
        parts.push(Doc::Space);
        parts.push(super::format_window_type(
            window,
            (cfg.line_length / 2).max(1),
            cfg,
            alias_tracker,
        ));
    }

    Doc::Group(parts)
}

fn table_factor_alias_str(factor: &TableFactor) -> Option<String> {
    match factor {
        TableFactor::Table { alias, .. }
        | TableFactor::Derived { alias, .. }
        | TableFactor::Function { alias, .. }
        | TableFactor::TableFunction { alias, .. }
        | TableFactor::UNNEST { alias, .. }
        | TableFactor::JsonTable { alias, .. }
        | TableFactor::OpenJsonTable { alias, .. }
        | TableFactor::NestedJoin { alias, .. }
        | TableFactor::Pivot { alias, .. }
        | TableFactor::Unpivot { alias, .. }
        | TableFactor::MatchRecognize { alias, .. }
        | TableFactor::XmlTable { alias, .. }
        | TableFactor::SemanticView { alias, .. } => alias.as_ref().map(|a| a.to_string()),
    }
}

fn format_join(
    join: &Join,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Doc {
    let (prefix, constraint, asof_match) = match &join.join_operator {
        JoinOperator::Join(constraint) => ("INNER JOIN", Some(constraint), None),
        JoinOperator::Inner(constraint) => ("INNER JOIN", Some(constraint), None),
        JoinOperator::Left(constraint) => ("LEFT JOIN", Some(constraint), None),
        JoinOperator::LeftOuter(constraint) => ("LEFT JOIN", Some(constraint), None),
        JoinOperator::Right(constraint) => ("RIGHT JOIN", Some(constraint), None),
        JoinOperator::RightOuter(constraint) => ("RIGHT JOIN", Some(constraint), None),
        JoinOperator::FullOuter(constraint) => ("FULL JOIN", Some(constraint), None),
        JoinOperator::Semi(constraint) => ("SEMI JOIN", Some(constraint), None),
        JoinOperator::LeftSemi(constraint) => ("LEFT SEMI JOIN", Some(constraint), None),
        JoinOperator::RightSemi(constraint) => ("RIGHT SEMI JOIN", Some(constraint), None),
        JoinOperator::Anti(constraint) => ("ANTI JOIN", Some(constraint), None),
        JoinOperator::LeftAnti(constraint) => ("LEFT ANTI JOIN", Some(constraint), None),
        JoinOperator::RightAnti(constraint) => ("RIGHT ANTI JOIN", Some(constraint), None),
        JoinOperator::CrossJoin(constraint) => ("CROSS JOIN", Some(constraint), None),
        JoinOperator::CrossApply => ("CROSS APPLY", None, None),
        JoinOperator::OuterApply => ("OUTER APPLY", None, None),
        JoinOperator::AsOf {
            match_condition,
            constraint,
        } => ("ASOF JOIN", Some(constraint), Some(match_condition)),
        JoinOperator::StraightJoin(constraint) => ("STRAIGHT_JOIN", Some(constraint), None),
    };

    let natural_prefix = matches!(constraint, Some(sqlparser::ast::JoinConstraint::Natural))
        .then_some("NATURAL")
        .unwrap_or("");

    let mut head = Vec::new();
    if !natural_prefix.is_empty() {
        head.push(Doc::Text(super::apply_keyword_case(natural_prefix, cfg)));
        head.push(Doc::Space);
    }
    head.push(Doc::Text(super::apply_keyword_case(prefix, cfg)));
    head.push(Doc::Space);
    head.push(format_table_factor(&join.relation, cfg, alias_tracker));

    let mut parts = vec![Doc::Group(head)];

    if let Some(expr) = asof_match {
        parts.push(Doc::Line);
        parts.push(Doc::Indent(Box::new(format_join_boolean_clause(
            "ASOF ON",
            expr,
            cfg,
            alias_tracker,
        ))));
    }

    if let Some(constraint) = constraint {
        match constraint {
            sqlparser::ast::JoinConstraint::On(expr) => {
                parts.push(Doc::Line);
                parts.push(Doc::Indent(Box::new(format_join_boolean_clause(
                    "ON",
                    expr,
                    cfg,
                    alias_tracker,
                ))));
            }
            sqlparser::ast::JoinConstraint::Using(cols) if !cols.is_empty() => {
                let cols = cols.iter().map(|c| c.to_string()).collect();
                parts.push(Doc::Space);
                parts.push(format_comma_clause("USING", cols, cfg));
            }
            sqlparser::ast::JoinConstraint::Using(_) => {}
            sqlparser::ast::JoinConstraint::Natural | sqlparser::ast::JoinConstraint::None => {}
        }
    }

    Doc::Group(parts)
}

pub(super) fn format_comma_clause(label: &str, items: Vec<String>, cfg: &FormatterConfig) -> Doc {
    let count = items.len();
    let max_len = items.iter().map(|s| s.len()).max().unwrap_or(0);
    let inline_len = label.len()
        + 1
        + items.iter().map(|s| s.len()).sum::<usize>()
        + (items.len().saturating_sub(1) * 2);

    let force_per_line = inline_len > (cfg.line_length as f32 * 0.7) as usize
        || count > 3
        || max_len > cfg.line_length / 3;

    let mut list_parts = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        list_parts.push(Doc::Text(item.clone()));
        if idx + 1 < items.len() {
            list_parts.push(Doc::Text(",".into()));
            if force_per_line {
                list_parts.push(Doc::Line);
            } else {
                list_parts.push(Doc::SoftLine);
            }
        }
    }
    let list = Doc::Group(list_parts);
    Doc::Group(vec![
        super::keyword_doc(cfg, label),
        Doc::Space,
        Doc::Indent(Box::new(list)),
    ])
}

pub(super) fn format_boolean_clause(
    label: &str,
    expr: &Expr,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Doc {
    let inline_limit = (cfg.line_length / 2).max(1);
    let cond = super::format_expr(expr, inline_limit, cfg, alias_tracker);
    Doc::Group(vec![
        super::keyword_doc(cfg, label),
        Doc::Space,
        Doc::Indent(Box::new(cond)),
    ])
}

pub(super) fn format_comma_clause_per_line(
    label: &str,
    items: Vec<String>,
    cfg: &FormatterConfig,
) -> Doc {
    let mut list_parts = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        list_parts.push(Doc::Text(item.clone()));
        if idx + 1 < items.len() {
            list_parts.push(Doc::Text(",".into()));
            list_parts.push(Doc::Line);
        }
    }
    let list = Doc::Group(list_parts);
    Doc::Group(vec![
        super::keyword_doc(cfg, label),
        Doc::Line,
        Doc::Indent(Box::new(list)),
    ])
}

fn format_join_boolean_clause(
    label: &str,
    expr: &Expr,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Doc {
    let inline_limit = (cfg.line_length / 2).max(1);
    let cond = super::format_expr(expr, inline_limit, cfg, alias_tracker);
    Doc::Group(vec![super::keyword_doc(cfg, label), Doc::Space, cond])
}
