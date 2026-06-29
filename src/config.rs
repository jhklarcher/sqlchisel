use clap::ValueEnum;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, ValueEnum, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KeywordCase {
    Upper,
    Lower,
    Capitalize,
}

#[derive(Debug, Clone, Copy, ValueEnum, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DialectKind {
    Ansi,
    Dremio,
}

#[derive(Debug, Clone, Copy, ValueEnum, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TemplatingMode {
    Passthrough,
    Dbt,
}

#[derive(Debug, Clone, Copy, ValueEnum, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SelectListStyle {
    Auto,
    PerLine,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FormatterConfig {
    pub line_length: usize,
    pub indent_width: usize,
    pub keyword_case: KeywordCase,
    pub dialect: DialectKind,
    pub templating: TemplatingMode,
    pub select_list_style: SelectListStyle,
    pub strict: bool,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            line_length: 100,
            indent_width: 2,
            keyword_case: KeywordCase::Upper,
            dialect: DialectKind::Ansi,
            templating: TemplatingMode::Passthrough,
            select_list_style: SelectListStyle::Auto,
            strict: false,
        }
    }
}
