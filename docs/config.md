# Configuration

`sqlchisel` reads formatter settings from `.sqlchisel.toml` and then applies CLI overrides.

## Resolution and Precedence

Resolution order:

1. If `--config <PATH>` is provided, that file is used.
2. Otherwise, `sqlchisel` looks for `.sqlchisel.toml` in the current working directory.
3. If no config file is found, built-in defaults are used.

Precedence:

1. Built-in defaults
2. Config file values
3. CLI flags (highest precedence)

## Example

```toml
line_length = 100
indent_width = 2
keyword_case = "upper"
dialect = "ansi"
templating = "passthrough"
select_list_style = "auto"
strict = false
```

## Keys

- `line_length` (`usize`): target print width. Default: `100`.
- `indent_width` (`usize`): spaces per indent level. Default: `2`.
- `keyword_case` (`"upper" | "lower" | "capitalize"`): keyword rendering. Default: `"upper"`.
- `dialect` (`"ansi" | "dremio"`): parser/formatter dialect. Default: `"ansi"`.
- `templating` (`"passthrough" | "dbt"`): template handling. Default: `"passthrough"`.
- `select_list_style` (`"auto" | "per_line"`): select-list layout strategy. Default: `"auto"`.
- `strict` (`bool`): fail on parse errors instead of raw fallback. Default: `false`.

## CLI Overrides

These flags override config values for the current run:

- `--line-length`
- `--indent-width`
- `--keyword-case`
- `--dialect`
- `--templating`
- `--select-list-style`
- `--strict` (enables strict mode)
