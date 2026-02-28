use anyhow::Result;
use sqlparser::keywords::Keyword;
use sqlparser::tokenizer::{Token, Tokenizer};

use crate::config::FormatterConfig;
use crate::format::doc::Doc;

pub(super) fn format_raw_sql(raw_sql: &str, cfg: &FormatterConfig) -> Result<Doc> {
    let dialect = super::dialect_for_kind(&cfg.dialect);
    let tokens = Tokenizer::new(dialect.as_ref(), raw_sql).tokenize()?;
    let mut parts = Vec::new();
    let mut pending_space = false;
    let mut prev_kind: Option<RawTokenKind> = None;

    for token in tokens {
        if matches!(token, Token::Whitespace(_)) {
            pending_space = true;
            continue;
        }

        let kind = RawTokenKind::from_token(&token);
        let starts_clause = kind.starts_new_clause(prev_kind);
        let mut insert_space = pending_space && should_insert_space(prev_kind, kind);

        if starts_clause && !parts.is_empty() {
            parts.push(Doc::Line);
            insert_space = false;
        }

        if insert_space {
            parts.push(Doc::Space);
        }

        let text = render_raw_token(&token, cfg);
        if !text.is_empty() {
            parts.push(Doc::Text(text));
        }

        pending_space = kind.needs_space_after();
        prev_kind = Some(kind);
    }

    if parts.is_empty() {
        Ok(Doc::Text(String::new()))
    } else {
        Ok(Doc::Group(parts))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RawTokenKind {
    Keyword(Keyword),
    WordOrLiteral,
    Comma,
    SemiColon,
    LParen,
    RParen,
    Period,
    DoubleColon,
    Other,
}

impl RawTokenKind {
    fn from_token(token: &Token) -> Self {
        match token {
            Token::Word(w) => {
                if w.keyword != Keyword::NoKeyword && w.quote_style.is_none() {
                    RawTokenKind::Keyword(w.keyword)
                } else {
                    RawTokenKind::WordOrLiteral
                }
            }
            Token::Number(_, _) => RawTokenKind::WordOrLiteral,
            Token::Comma => RawTokenKind::Comma,
            Token::SemiColon => RawTokenKind::SemiColon,
            Token::LParen => RawTokenKind::LParen,
            Token::RParen => RawTokenKind::RParen,
            Token::Period => RawTokenKind::Period,
            Token::DoubleColon => RawTokenKind::DoubleColon,
            _ => RawTokenKind::Other,
        }
    }

    fn starts_new_clause(self, prev: Option<RawTokenKind>) -> bool {
        match self {
            RawTokenKind::Keyword(kw) if is_clause_keyword(kw) => {
                if kw == Keyword::JOIN {
                    return !matches!(prev, Some(RawTokenKind::Keyword(prev_kw)) if is_join_modifier(prev_kw));
                }
                if is_join_modifier(kw) {
                    return !matches!(
                        prev,
                        Some(RawTokenKind::Keyword(prev_kw)) if is_join_modifier(prev_kw) || prev_kw == Keyword::JOIN
                    );
                }
                true
            }
            _ => false,
        }
    }

    fn needs_space_after(self) -> bool {
        !matches!(
            self,
            RawTokenKind::LParen
                | RawTokenKind::Period
                | RawTokenKind::DoubleColon
                | RawTokenKind::SemiColon
        )
    }
}

fn should_insert_space(prev: Option<RawTokenKind>, current: RawTokenKind) -> bool {
    match (prev, current) {
        (None, _) => false,
        (Some(RawTokenKind::Keyword(_)), RawTokenKind::LParen) => true,
        (_, RawTokenKind::Comma)
        | (_, RawTokenKind::Period)
        | (_, RawTokenKind::DoubleColon)
        | (_, RawTokenKind::RParen)
        | (_, RawTokenKind::SemiColon) => false,
        (_, RawTokenKind::LParen) => false,
        (Some(RawTokenKind::LParen), _)
        | (Some(RawTokenKind::Period), _)
        | (Some(RawTokenKind::DoubleColon), _) => false,
        _ => true,
    }
}

fn render_raw_token(token: &Token, cfg: &FormatterConfig) -> String {
    match token {
        Token::Word(w) if w.keyword != Keyword::NoKeyword && w.quote_style.is_none() => {
            super::apply_keyword_case(&w.value, cfg)
        }
        _ => token.to_string(),
    }
}

fn is_clause_keyword(keyword: Keyword) -> bool {
    matches!(
        keyword,
        Keyword::SELECT
            | Keyword::FROM
            | Keyword::WHERE
            | Keyword::GROUP
            | Keyword::HAVING
            | Keyword::WINDOW
            | Keyword::ORDER
            | Keyword::LIMIT
            | Keyword::OFFSET
            | Keyword::JOIN
            | Keyword::INNER
            | Keyword::LEFT
            | Keyword::RIGHT
            | Keyword::FULL
            | Keyword::CROSS
            | Keyword::UNION
            | Keyword::EXCEPT
            | Keyword::INTERSECT
            | Keyword::VALUES
            | Keyword::SET
            | Keyword::INSERT
            | Keyword::UPDATE
            | Keyword::DELETE
            | Keyword::WITH
            | Keyword::ON
            | Keyword::CREATE
            | Keyword::ALTER
            | Keyword::PARTITION
            | Keyword::USING
    )
}

fn is_join_modifier(keyword: Keyword) -> bool {
    matches!(
        keyword,
        Keyword::INNER | Keyword::LEFT | Keyword::RIGHT | Keyword::FULL | Keyword::CROSS
    )
}
