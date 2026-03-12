use sqlparser::ast::{CaseWhen, Expr};

use crate::config::FormatterConfig;
use crate::format::doc::Doc;

pub(super) fn format_case(
    operand: Option<&Expr>,
    branches: &[CaseWhen],
    else_result: &Option<Box<Expr>>,
    inline_limit: usize,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Doc {
    let branch_count = branches.len();
    let inline_len =
        estimate_case_inline_length(operand, branches, else_result).unwrap_or(usize::MAX);
    let force_multiline = branch_count > 1 || inline_len > inline_limit;

    if force_multiline {
        format_case_multiline(
            operand,
            branches,
            else_result,
            inline_limit,
            cfg,
            alias_tracker,
        )
    } else {
        format_case_inline(
            operand,
            branches,
            else_result,
            inline_limit,
            cfg,
            alias_tracker,
        )
    }
}

fn format_case_inline(
    operand: Option<&Expr>,
    branches: &[CaseWhen],
    else_result: &Option<Box<Expr>>,
    inline_limit: usize,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Doc {
    let mut parts = vec![super::keyword_doc(cfg, "CASE")];

    if let Some(op) = operand {
        parts.push(Doc::Space);
        parts.push(super::format_expr(op, inline_limit, cfg, alias_tracker));
    }

    for branch in branches {
        parts.push(Doc::Space);
        parts.push(super::keyword_doc(cfg, "WHEN"));
        parts.push(Doc::Space);
        parts.push(super::format_expr(
            &branch.condition,
            inline_limit,
            cfg,
            alias_tracker,
        ));
        parts.push(Doc::Space);
        parts.push(super::keyword_doc(cfg, "THEN"));
        parts.push(Doc::Space);
        parts.push(super::format_expr(
            &branch.result,
            inline_limit,
            cfg,
            alias_tracker,
        ));
    }

    if let Some(res) = else_result {
        parts.push(Doc::Space);
        parts.push(super::keyword_doc(cfg, "ELSE"));
        parts.push(Doc::Space);
        parts.push(super::format_expr(res, inline_limit, cfg, alias_tracker));
    }

    parts.push(Doc::Space);
    parts.push(super::keyword_doc(cfg, "END"));

    Doc::Group(parts)
}

fn format_case_multiline(
    operand: Option<&Expr>,
    branches: &[CaseWhen],
    else_result: &Option<Box<Expr>>,
    inline_limit: usize,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Doc {
    let mut head = vec![super::keyword_doc(cfg, "CASE")];
    if let Some(op) = operand {
        head.push(Doc::Space);
        head.push(super::format_expr(op, inline_limit, cfg, alias_tracker));
    }

    let mut lines = Vec::new();
    for (idx, branch) in branches.iter().enumerate() {
        if idx > 0 {
            lines.push(Doc::Line);
        }
        lines.push(Doc::Group(vec![
            super::keyword_doc(cfg, "WHEN"),
            Doc::Space,
            super::format_expr(&branch.condition, inline_limit, cfg, alias_tracker),
            Doc::Space,
            super::keyword_doc(cfg, "THEN"),
            Doc::Space,
            super::format_expr(&branch.result, inline_limit, cfg, alias_tracker),
        ]));
    }

    if let Some(res) = else_result {
        if !lines.is_empty() {
            lines.push(Doc::Line);
        }
        lines.push(Doc::Group(vec![
            super::keyword_doc(cfg, "ELSE"),
            Doc::Space,
            super::format_expr(res, inline_limit, cfg, alias_tracker),
        ]));
    }

    let mut parts = Vec::new();
    parts.push(Doc::Group(head));

    if !lines.is_empty() {
        parts.push(Doc::Line);
        parts.push(Doc::Indent(Box::new(Doc::Group(lines))));
    }

    parts.push(Doc::Line);
    parts.push(super::keyword_doc(cfg, "END"));

    Doc::Group(parts)
}

fn estimate_case_inline_length(
    operand: Option<&Expr>,
    branches: &[CaseWhen],
    else_result: &Option<Box<Expr>>,
) -> Option<usize> {
    let mut len = "CASE".len();

    if let Some(op) = operand {
        len = len.checked_add(1 + op.to_string().len())?;
    }

    for branch in branches {
        len = len
            .checked_add(1 + "WHEN".len() + 1 + branch.condition.to_string().len())?
            .checked_add(1 + "THEN".len() + 1 + branch.result.to_string().len())?;
    }

    if let Some(res) = else_result {
        len = len.checked_add(1 + "ELSE".len() + 1 + res.to_string().len())?;
    }

    len.checked_add(1 + "END".len())
}
