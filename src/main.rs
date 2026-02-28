use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;

use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;

mod config;
mod format;
mod parser;
use crate::config::{DialectKind, FormatterConfig, KeywordCase, SelectListStyle};
use crate::format::sql::format_sql;
use crate::parser::{parse_sql_with_options, ParseOptions, ParsedStatement};

const DEFAULT_CONFIG: &str = ".sqlchisel.toml";

#[derive(Debug, Parser)]
#[command(
    name = "sqlchisel",
    about = "SQL formatter (MVP pass-through)",
    version
)]
struct Cli {
    /// SQL files to read. Use --stdin to read from standard input.
    #[arg(value_name = "FILES", value_hint = clap::ValueHint::FilePath)]
    files: Vec<PathBuf>,

    /// Read SQL from standard input.
    #[arg(long)]
    stdin: bool,

    /// Optional path to a config file (defaults to .sqlchisel.toml when present).
    #[arg(long, value_name = "PATH", value_hint = clap::ValueHint::FilePath)]
    config: Option<PathBuf>,

    /// Override line length (characters).
    #[arg(long, value_name = "N")]
    line_length: Option<usize>,

    /// Override indent width (spaces).
    #[arg(long, value_name = "N")]
    indent_width: Option<usize>,

    /// Override keyword case.
    #[arg(long, value_enum)]
    keyword_case: Option<KeywordCase>,

    /// Override dialect.
    #[arg(long, value_enum)]
    dialect: Option<DialectKind>,

    /// Override select list style.
    #[arg(long, value_enum)]
    select_list_style: Option<SelectListStyle>,

    /// Error on parse failures instead of attempting best-effort formatting.
    #[arg(long)]
    strict: bool,

    /// Parse inputs and print the AST for debugging.
    #[arg(long)]
    debug_parse: bool,

    /// Format input instead of echoing as-is.
    #[arg(long)]
    format: bool,

    /// Check if inputs are formatted; exit 1 if changes are needed.
    #[arg(long)]
    check: bool,

    /// Rewrite input files in place with formatted output.
    #[arg(long)]
    write: bool,

    /// Include glob(s) when recursing directories (default: **/*.sql).
    #[arg(long, value_name = "GLOB")]
    include: Vec<String>,

    /// Exclude glob(s) when recursing directories.
    #[arg(long, value_name = "GLOB")]
    exclude: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    PassThrough,
    FormatStdout,
    Check,
    Write,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut config = load_config(cli.config.as_deref())?;
    apply_cli_overrides(&mut config, &cli);

    let mode = choose_mode(&cli)?;

    if cli.stdin && !cli.files.is_empty() {
        anyhow::bail!("use either FILES or --stdin, not both");
    }

    let mut any_changed = false;

    if cli.stdin {
        let sql = read_stdin()?;
        let changed = handle_input("<stdin>", None, &sql, &config, cli.debug_parse, mode)?;
        any_changed |= changed;
    } else {
        if cli.files.is_empty() {
            anyhow::bail!("no input provided; pass FILES or --stdin");
        }

        let inputs = collect_input_files(&cli.files, &cli.include, &cli.exclude)?;

        if inputs.is_empty() {
            anyhow::bail!("no input files found (looking for *.sql)");
        }

        for path in &inputs {
            let sql = fs::read_to_string(path)
                .with_context(|| format!("failed to read input file {:?}", path))?;
            let changed = handle_input(
                &path.display().to_string(),
                Some(path.as_path()),
                &sql,
                &config,
                cli.debug_parse,
                mode,
            )?;
            any_changed |= changed;
        }
    }

    if matches!(mode, Mode::Check) && any_changed {
        process::exit(1);
    }

    Ok(())
}

fn load_config(path: Option<&Path>) -> Result<FormatterConfig> {
    let resolved = path.unwrap_or_else(|| Path::new(DEFAULT_CONFIG));

    if !resolved.exists() {
        return Ok(FormatterConfig::default());
    }

    let contents = fs::read_to_string(resolved)
        .with_context(|| format!("failed to read config file {:?}", resolved))?;

    let config: FormatterConfig = toml::from_str(&contents)
        .with_context(|| format!("failed to parse config file {:?}", resolved))?;

    Ok(config)
}

fn apply_cli_overrides(config: &mut FormatterConfig, cli: &Cli) {
    if let Some(line_length) = cli.line_length {
        config.line_length = line_length;
    }
    if let Some(indent_width) = cli.indent_width {
        config.indent_width = indent_width;
    }
    if let Some(keyword_case) = cli.keyword_case {
        config.keyword_case = keyword_case;
    }
    if let Some(dialect) = cli.dialect {
        config.dialect = dialect;
    }
    if let Some(select_list_style) = cli.select_list_style {
        config.select_list_style = select_list_style;
    }
    if cli.strict {
        config.strict = true;
    }
}

fn read_stdin() -> Result<String> {
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .context("failed to read from stdin")?;
    Ok(buf)
}

fn choose_mode(cli: &Cli) -> Result<Mode> {
    let mut selected = Mode::PassThrough;
    let mut selected_count = 0;

    if cli.format {
        selected = Mode::FormatStdout;
        selected_count += 1;
    }
    if cli.check {
        selected = Mode::Check;
        selected_count += 1;
    }
    if cli.write {
        selected = Mode::Write;
        selected_count += 1;
    }

    if selected_count > 1 {
        anyhow::bail!("use only one of --format, --check, or --write");
    }

    if cli.write && cli.stdin {
        anyhow::bail!("--write requires file inputs (not --stdin)");
    }

    Ok(selected)
}

fn collect_input_files(
    paths: &[PathBuf],
    includes: &[String],
    excludes: &[String],
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    let matcher = build_matchers(includes, excludes)?;
    for path in paths {
        gather_paths(path, &mut files, &mut seen, &matcher)?;
    }
    Ok(files)
}

fn gather_paths(
    path: &Path,
    files: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
    matcher: &GlobMatchers,
) -> Result<()> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !seen.insert(canonical.clone()) {
        return Ok(());
    }

    let meta = fs::symlink_metadata(path)
        .with_context(|| format!("failed to stat input path {:?}", path.display()))?;
    if meta.file_type().is_symlink() {
        return Ok(());
    }

    if meta.is_file() {
        if matcher.matches(path) {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }

    if meta.is_dir() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if is_ignored_dir(name) {
            return Ok(());
        }
        if matcher.is_excluded(path) {
            return Ok(());
        }
        for entry in fs::read_dir(path)
            .with_context(|| format!("failed to read directory {:?}", path.display()))?
        {
            let entry = entry?;
            let child_path = entry.path();
            gather_paths(&child_path, files, seen, matcher)?;
        }
    }

    Ok(())
}

fn is_ignored_dir(name: &str) -> bool {
    matches!(name, ".git" | "target" | ".cargo")
}

fn build_matchers(includes: &[String], excludes: &[String]) -> Result<GlobMatchers> {
    let include_patterns = if includes.is_empty() {
        vec![glob_to_regex("**/*.sql")?]
    } else {
        includes
            .iter()
            .map(|p| glob_to_regex(p))
            .collect::<Result<Vec<_>>>()?
    };

    let exclude_patterns = excludes
        .iter()
        .map(|p| glob_to_regex(p))
        .collect::<Result<Vec<_>>>()?;

    Ok(GlobMatchers {
        include: include_patterns,
        exclude: exclude_patterns,
    })
}

struct GlobMatchers {
    include: Vec<Regex>,
    exclude: Vec<Regex>,
}

impl GlobMatchers {
    fn matches(&self, path: &Path) -> bool {
        let path_str = normalize_path(path);
        if self.is_excluded(path) {
            return false;
        }
        self.include.iter().any(|re| re.is_match(&path_str))
    }

    fn is_excluded(&self, path: &Path) -> bool {
        let path_str = normalize_path(path);
        self.exclude.iter().any(|re| re.is_match(&path_str))
    }
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn glob_to_regex(pattern: &str) -> Result<Regex> {
    let mut regex = String::from("^");
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if matches!(chars.peek(), Some('*')) {
                    chars.next();
                    regex.push_str(".*");
                } else {
                    regex.push_str("[^/]*");
                }
            }
            '?' => regex.push_str("[^/]"),
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '{' | '}' | '[' | ']' | '\\' => {
                regex.push('\\');
                regex.push(c);
            }
            '/' => regex.push('/'),
            _ => regex.push(c),
        }
    }
    regex.push('$');
    Ok(Regex::new(&regex)?)
}

fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    let mut tmp = path.to_path_buf();
    let mut ext = tmp
        .extension()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "".into());
    if !ext.is_empty() {
        ext.push('.');
    }
    ext.push_str("sqlchisel.tmp");
    tmp.set_extension(ext);

    fs::write(&tmp, contents)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn handle_input(
    label: &str,
    path: Option<&Path>,
    sql: &str,
    config: &FormatterConfig,
    debug_parse: bool,
    mode: Mode,
) -> Result<bool> {
    if debug_parse {
        let ast = parse_sql_with_options(
            sql,
            config.dialect,
            ParseOptions {
                strict: config.strict,
            },
        )
        .with_context(|| format!("failed to parse input {label}"))?;
        eprintln!("-- Debug AST for {label} --");
        for stmt in &ast {
            match stmt {
                ParsedStatement::Sql {
                    stmt,
                    version,
                    has_semicolon,
                    relation_alias_has_as,
                } => {
                    eprintln!("{stmt:#?}");
                    eprintln!("  -- has_semicolon: {has_semicolon}");
                    eprintln!(
                        "  -- relation_alias_has_as ({} entries)",
                        relation_alias_has_as.len()
                    );
                    if let Some(v) = version {
                        eprintln!("  -- Dremio version: {v:#?}");
                    }
                }
                ParsedStatement::Command { cmd, has_semicolon } => {
                    eprintln!("DremioCommand: {cmd:#?}, has_semicolon: {has_semicolon}");
                }
                ParsedStatement::Raw { sql, has_semicolon } => {
                    eprintln!("Raw SQL (parse fallback): {sql}");
                    eprintln!("  -- has_semicolon: {has_semicolon}");
                }
            }
        }
    }

    match mode {
        Mode::PassThrough => {
            print!("{sql}");
            Ok(false)
        }
        Mode::FormatStdout => {
            let formatted = format_sql(sql, config)
                .with_context(|| format!("failed to format input {label}"))?;
            print!("{formatted}");
            Ok(false)
        }
        Mode::Check => {
            let formatted = format_sql(sql, config)
                .with_context(|| format!("failed to format input {label}"))?;
            let changed = formatted != sql;
            if changed {
                eprintln!("{label} would be reformatted");
            }
            Ok(changed)
        }
        Mode::Write => {
            let path = path.ok_or_else(|| anyhow::anyhow!("--write requires file inputs"))?;
            let formatted = format_sql(sql, config)
                .with_context(|| format!("failed to format input {label}"))?;
            if formatted != sql {
                write_atomic(path, formatted.as_bytes())
                    .with_context(|| format!("failed to write formatted output for {label}"))?;
                eprintln!("wrote formatted {label}");
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }
}
