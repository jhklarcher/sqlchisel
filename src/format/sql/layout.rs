use sqlparser::ast::{Join, JoinOperator, Select, SelectItem};

use crate::config::{FormatterConfig, SelectListStyle};

pub(super) fn choose_layout(select: &Select, cfg: &FormatterConfig) -> super::SelectLayout {
    if cfg.select_list_style == SelectListStyle::PerLine {
        return super::SelectLayout::PerLine;
    }

    if select.projection.len() == 1
        && matches!(
            select.projection[0],
            SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard { .. }
        )
    {
        return super::SelectLayout::Inline;
    }

    let inline_len = estimate_first_line_length(select);
    if inline_len <= cfg.line_length {
        super::SelectLayout::Inline
    } else {
        super::SelectLayout::PerLine
    }
}

fn estimate_first_line_length(select: &Select) -> usize {
    let mut len = "SELECT".len();
    if select.distinct.is_some() {
        len += 1 + "DISTINCT".len();
    }
    len += 1; // space before select list
    len += estimate_projection_length(&select.projection);

    if let Some(first_from) = select.from.first() {
        len += 1; // space before FROM
        len += "FROM ".len();
        len += first_from.relation.to_string().len();

        if let Some(first_join) = first_from.joins.first() {
            len += 1; // space before JOIN
            len += join_prefix_len(first_join);
            len += first_join.relation.to_string().len();
        }
    }

    len
}

fn estimate_projection_length(items: &[SelectItem]) -> usize {
    let mut len = 0;
    for (idx, item) in items.iter().enumerate() {
        len += item.to_string().len();
        if idx + 1 < items.len() {
            len += 2; // comma + space
        }
    }
    len
}

fn join_prefix_len(join: &Join) -> usize {
    let (prefix, constraint) = match &join.join_operator {
        JoinOperator::Inner(constraint) => ("INNER JOIN", Some(constraint)),
        JoinOperator::LeftOuter(constraint) => ("LEFT JOIN", Some(constraint)),
        JoinOperator::RightOuter(constraint) => ("RIGHT JOIN", Some(constraint)),
        JoinOperator::FullOuter(constraint) => ("FULL JOIN", Some(constraint)),
        JoinOperator::LeftSemi(constraint) => ("LEFT SEMI JOIN", Some(constraint)),
        JoinOperator::RightSemi(constraint) => ("RIGHT SEMI JOIN", Some(constraint)),
        JoinOperator::LeftAnti(constraint) => ("LEFT ANTI JOIN", Some(constraint)),
        JoinOperator::RightAnti(constraint) => ("RIGHT ANTI JOIN", Some(constraint)),
        JoinOperator::CrossJoin => ("CROSS JOIN", None),
        JoinOperator::CrossApply => ("CROSS APPLY", None),
        JoinOperator::OuterApply => ("OUTER APPLY", None),
        JoinOperator::AsOf { .. } => ("ASOF JOIN", None),
    };

    let natural_len = matches!(constraint, Some(sqlparser::ast::JoinConstraint::Natural))
        .then(|| "NATURAL ".len())
        .unwrap_or(0);

    natural_len + prefix.len() + 1 // trailing space before relation
}
