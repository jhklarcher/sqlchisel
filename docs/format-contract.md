# Format Contract

This document defines the stable behavior contract for `sqlchisel`. It focuses on guarantees and intentionally avoids layout heuristics that may change over time.

## Scope

The contract covers:

- output stability and repeatability
- statement and semicolon handling
- parser fallback behavior (`Raw` vs `--strict`)
- CLI mode exclusivity and check-mode exit semantics
- templated SQL passthrough (Jinja markers)

Heuristics such as select-list thresholds and line-break preferences are documented in [`style-guide.md`](style-guide.md) and are not part of this contract.

## Determinism and Idempotence

- For a given `sqlchisel` version and the same effective configuration, formatting is deterministic.
- Formatting an already-formatted input should produce the same output (idempotent), except when a bug is present.
- This guarantee is per version. Layout details may change across versions when formatter heuristics improve.

## Statement Separation

- Top-level SQL statements are emitted in input order.
- Consecutive top-level statements are separated by exactly one blank line in formatted output.

## Semicolon Preservation

- `sqlchisel` preserves whether each top-level statement fragment had a trailing semicolon.
- It does not add semicolons to statements that did not end with one.
- It does not remove semicolons from statements that did end with one.

## String Literal Preservation

- Single-quoted string literal contents are preserved as written, including doubled quotes and over-escaped forms.
- Quoted identifier contents are not recased.

## JOIN Token Preservation

- `JOIN` stays `JOIN`.
- `INNER JOIN` stays `INNER JOIN`.

## Parse Fallback vs Strict Mode

Default behavior (`strict = false` / no `--strict`):

- If a top-level fragment cannot be parsed, `sqlchisel` falls back to a token-level raw formatter for that fragment instead of failing the whole input.
- Other fragments in the same input can still be fully formatted.
- The stable guarantee is the fallback behavior (no hard failure by default), not the exact token-level layout chosen for unsupported syntax across versions.

Strict behavior (`strict = true` or `--strict`):

- Parse failures are returned as errors.
- Formatting/check/write fails instead of using raw fallback.

## Jinja Passthrough

- By default, if the input contains Jinja markers (`{{`, `{%`, or `{#`), `sqlchisel` returns the input unchanged.
- This passthrough behavior is part of the current stable contract to avoid corrupting templated SQL.
- When `templating = "dbt"` or `--templating dbt` is enabled, dbt/Jinja tags are protected and restored while surrounding SQL is formatted.

## CLI Mode Contract

- Only one of `--format`, `--check`, or `--write` may be used in a single invocation.
- `--write` cannot be combined with `--stdin`.
- File paths and `--stdin` are mutually exclusive.
- In `--check` mode, `sqlchisel` exits with status `1` if any input would be reformatted.

## CLI Exit Codes and Error Behavior (Examples)

`--check` mode:

- Exit `0` when all inputs are already formatted.
- Exit `1` when any input differs from formatted output; `sqlchisel` prints `<path> would be reformatted` to stderr for changed inputs.

Invalid mode/input combinations (error, non-zero exit):

- `sqlchisel --format --check file.sql`
  - error: `use only one of --format, --check, or --write`
- `sqlchisel --write --stdin`
  - error: `--write requires file inputs (not --stdin)`
- `sqlchisel --stdin file.sql`
  - error: `use either FILES or --stdin, not both`
- `sqlchisel` (with no files and no `--stdin`)
  - error: `no input provided; pass FILES or --stdin`

Notes:

- Exact parser/IO error wording outside the cases above is not a stable contract.
- `--strict` changes parse-failure behavior from raw fallback to an error (non-zero exit).

## Config Precedence

- Effective configuration is resolved as: defaults -> config file -> CLI overrides.
- `--config <PATH>` overrides the default config file discovery path.

## Fixture-Backed Contract Examples

Statement spacing + semicolon preservation:

- `fixtures/dremio/in/13.sql` contains three top-level statements that end with semicolons.
- `fixtures/dremio/expected/13.sql` preserves those semicolons and emits exactly one blank line between each formatted statement.

Raw fallback (default non-strict mode):

- `fixtures/ansi/in/11.sql` contains unsupported syntax (`unknown_verb ...`) that does not parse as a normal SQL statement.
- `fixtures/ansi/expected/11.sql` shows `sqlchisel` still returns formatted output for that fragment (keyword casing/spacing applied) instead of failing.

## Explicitly Non-Contract (Subject to Change)

- Select-list layout thresholds and line-break heuristics
- Clause wrapping decisions when multiple layouts are valid
- Best-effort comment placement in ambiguous locations
- Exact raw-fallback formatting details for unsupported syntax
