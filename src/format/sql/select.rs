use sqlparser::ast::{Select, SelectItem};

use crate::config::FormatterConfig;
use crate::format::doc::Doc;

pub(super) fn format_select(
    select: &Select,
    layout: super::SelectLayout,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Doc {
    let select_list = format_projection(&select.projection, layout, cfg, alias_tracker);

    let mut parts = Vec::new();
    parts.push(super::keyword_doc(cfg, "SELECT"));

    if select.distinct.is_some() {
        parts.push(Doc::Space);
        parts.push(super::keyword_doc(cfg, "DISTINCT"));
        match layout {
            super::SelectLayout::Inline => parts.push(Doc::Space),
            _ => parts.push(Doc::Line),
        }
    } else {
        match layout {
            super::SelectLayout::Inline => parts.push(Doc::Space),
            _ => parts.push(Doc::Line),
        }
    }

    let indented = Doc::Indent(Box::new(select_list));
    parts.push(indented);

    Doc::Group(parts)
}

fn format_projection(
    items: &[SelectItem],
    layout: super::SelectLayout,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Doc {
    let inline_limit = (cfg.line_length / 2).max(20);
    match layout {
        super::SelectLayout::Inline => join_inline(items, inline_limit, cfg, alias_tracker),
        super::SelectLayout::PerLine => join_per_line(items, inline_limit, cfg, alias_tracker),
    }
}

fn join_inline(
    items: &[SelectItem],
    inline_limit: usize,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Doc {
    let mut parts = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        parts.push(format_select_item(item, inline_limit, cfg, alias_tracker));
        if idx + 1 < items.len() {
            parts.push(Doc::Text(",".into()));
            parts.push(Doc::Space);
        }
    }
    Doc::Group(parts)
}

fn join_per_line(
    items: &[SelectItem],
    inline_limit: usize,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Doc {
    let mut parts = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        parts.push(format_select_item(item, inline_limit, cfg, alias_tracker));
        if idx + 1 < items.len() {
            parts.push(Doc::Text(",".into()));
            parts.push(Doc::Line);
        }
    }
    Doc::Group(parts)
}

fn format_select_item(
    item: &SelectItem,
    inline_limit: usize,
    cfg: &FormatterConfig,
    alias_tracker: &mut super::RelationAliasTracker,
) -> Doc {
    match item {
        SelectItem::UnnamedExpr(expr) => super::format_expr(expr, inline_limit, cfg, alias_tracker),
        SelectItem::ExprWithAlias { expr, alias } => Doc::Group(vec![
            super::format_expr(expr, inline_limit, cfg, alias_tracker),
            Doc::Space,
            super::keyword_doc(cfg, "AS"),
            Doc::Space,
            Doc::Text(alias.to_string()),
        ]),
        other => Doc::Text(other.to_string()),
    }
}
