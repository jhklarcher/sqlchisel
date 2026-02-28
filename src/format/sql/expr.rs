use sqlparser::ast::{BinaryOperator, Expr};

use crate::config::FormatterConfig;
use crate::format::doc::Doc;

pub(super) fn format_expr(
    expr: &Expr,
    inline_limit: usize,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Doc {
    match expr {
        Expr::Value(v) => super::format_value_literal(v, cfg),
        Expr::Function(fun) => super::format_function_call(fun, inline_limit, cfg, alias_tracker),
        Expr::Cast {
            kind,
            expr,
            data_type,
            format,
            ..
        } => super::format_cast_expr(
            kind,
            expr,
            data_type,
            format,
            inline_limit,
            cfg,
            alias_tracker,
        ),
        Expr::TypedString(typed) => super::format_typed_string(typed, cfg),
        Expr::Interval(interval) => {
            super::format_interval_expr(interval, inline_limit, cfg, alias_tracker)
        }
        Expr::IsNull(inner) => {
            super::format_unary_predicate(inner, "IS NULL", inline_limit, cfg, alias_tracker)
        }
        Expr::IsNotNull(inner) => {
            super::format_unary_predicate(inner, "IS NOT NULL", inline_limit, cfg, alias_tracker)
        }
        Expr::IsTrue(inner) => {
            super::format_unary_predicate(inner, "IS TRUE", inline_limit, cfg, alias_tracker)
        }
        Expr::IsNotTrue(inner) => {
            super::format_unary_predicate(inner, "IS NOT TRUE", inline_limit, cfg, alias_tracker)
        }
        Expr::IsFalse(inner) => {
            super::format_unary_predicate(inner, "IS FALSE", inline_limit, cfg, alias_tracker)
        }
        Expr::IsNotFalse(inner) => {
            super::format_unary_predicate(inner, "IS NOT FALSE", inline_limit, cfg, alias_tracker)
        }
        Expr::IsUnknown(inner) => {
            super::format_unary_predicate(inner, "IS UNKNOWN", inline_limit, cfg, alias_tracker)
        }
        Expr::IsNotUnknown(inner) => {
            super::format_unary_predicate(inner, "IS NOT UNKNOWN", inline_limit, cfg, alias_tracker)
        }
        Expr::BinaryOp { left, op, right } => match op {
            BinaryOperator::And | BinaryOperator::Or => {
                let op_str = super::apply_keyword_case(&format!("{op}"), cfg);
                let left_doc = format_expr(left, inline_limit, cfg, alias_tracker);
                let right_doc = format_expr(right, inline_limit, cfg, alias_tracker);
                Doc::Group(vec![Doc::Group(vec![
                    left_doc,
                    Doc::Line,
                    Doc::Text(op_str.clone()),
                    Doc::Space,
                    right_doc.clone(),
                ])])
            }
            _ => {
                let left_doc = format_expr(left, inline_limit, cfg, alias_tracker);
                let right_doc = format_expr(right, inline_limit, cfg, alias_tracker);
                let op_str = super::apply_keyword_case(&op.to_string(), cfg);
                Doc::Group(vec![
                    left_doc,
                    Doc::Space,
                    Doc::Text(op_str),
                    Doc::Space,
                    right_doc,
                ])
            }
        },
        Expr::Nested(e) => {
            let inner = format_expr(e, inline_limit, cfg, alias_tracker);
            let inline_len = e.to_string().len() + 2;
            if inline_len <= inline_limit && !contains_logical_ops(e) {
                Doc::Group(vec![Doc::Text("(".into()), inner, Doc::Text(")".into())])
            } else {
                Doc::Group(vec![
                    Doc::Text("(".into()),
                    Doc::Line,
                    Doc::Indent(Box::new(inner)),
                    Doc::Line,
                    Doc::Text(")".into()),
                ])
            }
        }
        Expr::Case {
            operand,
            conditions,
            else_result,
            ..
        } => super::case::format_case(
            operand.as_deref(),
            conditions,
            else_result,
            inline_limit,
            cfg,
            alias_tracker,
        ),
        Expr::Exists { subquery, negated } => {
            let formatted = super::format_query(subquery, cfg, None, alias_tracker)
                .unwrap_or_else(|_| Doc::Text(subquery.to_string()));
            let keyword = if *negated { "NOT EXISTS" } else { "EXISTS" };
            let inline_len = subquery.to_string().len() + keyword.len() + 3;
            if inline_len <= inline_limit {
                return Doc::Group(vec![
                    super::keyword_doc(cfg, keyword),
                    Doc::Space,
                    Doc::Text("(".into()),
                    formatted,
                    Doc::Text(")".into()),
                ]);
            }
            Doc::Group(vec![
                super::keyword_doc(cfg, keyword),
                Doc::Line,
                Doc::Indent(Box::new(Doc::Group(vec![
                    Doc::Text("(".into()),
                    Doc::Line,
                    Doc::Indent(Box::new(formatted)),
                    Doc::Line,
                    Doc::Text(")".into()),
                ]))),
            ])
        }
        Expr::Subquery(query) => {
            let formatted = super::format_query(query, cfg, None, alias_tracker)
                .unwrap_or_else(|_| Doc::Text(query.to_string()));
            let inline = query.to_string();
            let inline_len = inline.len() + 2;
            if inline_len <= inline_limit {
                return Doc::Group(vec![
                    Doc::Text("(".into()),
                    formatted,
                    Doc::Text(")".into()),
                ]);
            }
            Doc::Group(vec![
                Doc::Text("(".into()),
                Doc::Line,
                Doc::Indent(Box::new(formatted)),
                Doc::Line,
                Doc::Text(")".into()),
            ])
        }
        _ => Doc::Text(expr.to_string()),
    }
}

fn contains_logical_ops(expr: &Expr) -> bool {
    match expr {
        Expr::BinaryOp { op, left, right } => {
            matches!(op, BinaryOperator::And | BinaryOperator::Or)
                || contains_logical_ops(left)
                || contains_logical_ops(right)
        }
        Expr::Nested(inner) => contains_logical_ops(inner),
        Expr::Between {
            expr, low, high, ..
        } => contains_logical_ops(expr) || contains_logical_ops(low) || contains_logical_ops(high),
        Expr::Case {
            operand,
            conditions,
            else_result,
            ..
        } => {
            operand
                .as_deref()
                .map(contains_logical_ops)
                .unwrap_or(false)
                || conditions.iter().any(|branch| {
                    contains_logical_ops(&branch.condition) || contains_logical_ops(&branch.result)
                })
                || else_result
                    .as_deref()
                    .map(contains_logical_ops)
                    .unwrap_or(false)
        }
        _ => false,
    }
}
