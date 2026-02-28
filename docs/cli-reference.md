# CLI Reference

`sqlchisel` formats SQL files or stdin with ANSI and Dremio dialect support.

## Modes

Only one of these may be selected at a time:

- `--format`: format input and print to stdout
- `--check`: compare formatted output to input; exits `1` if any input would change
- `--write`: rewrite files in place

If none of the mode flags are provided, `sqlchisel` currently passes input through unchanged.

## Input Selection

- File inputs: `sqlchisel --format path/to/file.sql`
- Stdin input: `cat query.sql | sqlchisel --stdin --format`

Constraints:

- Use either file paths or `--stdin`, not both.
- `--write` requires file inputs and cannot be combined with `--stdin`.
- At least one input source is required.

## Common Flags

- `--dialect <ansi|dremio>`
- `--keyword-case <upper|lower|capitalize>`
- `--line-length <N>`
- `--indent-width <N>`
- `--select-list-style <auto|per_line>`
- `--strict` (fail on parse errors instead of raw fallback)
- `--config <PATH>` (explicit config file)
- `--include <GLOB>` / `--exclude <GLOB>` (directory recursion filters)
- `--debug-parse` (print parsed AST/debug metadata)

## Directory Traversal

When a provided path is a directory, `sqlchisel` recursively collects matching files.

- Default include glob: `**/*.sql`
- `--include` overrides the default include list
- `--exclude` filters paths out
- Ignored directories during recursion: `.git`, `target`, `.cargo`
- Symlinks are skipped

## Exit Behavior

- Success: exit `0`
- `--check` with any file needing changes: exit `1`
- Invalid CLI combinations, parse failures in strict mode, read/write errors, and other runtime failures return non-zero errors

See [`format-contract.md`](format-contract.md) for stable behavior expectations around formatting output.
