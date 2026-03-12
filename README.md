# sqlchisel

Rust CLI formatter for ANSI SQL with first-class Dremio support. Output is deterministic and intended for CI, editor, and pre-commit use.

## Install

```bash
git clone https://github.com/jhklarcher/sqlchisel.git
cd sqlchisel
cargo install --path .
```

## Core Workflows

```bash
# format to stdout
sqlchisel --format path/to/query.sql

# check formatting (exit 1 if changes are needed)
sqlchisel --check path/to/query.sql

# rewrite files in place
sqlchisel --write path/to/query.sql

# format from stdin
cat query.sql | sqlchisel --stdin --format
```

## Highlights

- ANSI SQL and Dremio dialect support (`--dialect ansi|dremio`)
- Config file plus CLI overrides (`.sqlchisel.toml`)
- Directory recursion with `--include` / `--exclude`
- Best-effort raw fallback on parse failures (or fail-fast with `--strict`)

## Docs

- Docs index: [`docs/README.md`](docs/README.md)
- CLI reference: [`docs/cli-reference.md`](docs/cli-reference.md)
- Config: [`docs/config.md`](docs/config.md)
- Stable formatter behavior: [`docs/format-contract.md`](docs/format-contract.md)
- Formatting heuristics (non-contract): [`docs/style-guide.md`](docs/style-guide.md)
- Dremio support notes: [`docs/dremio.md`](docs/dremio.md)
- Dremio reference coverage tracker: [`docs/dremio-support-matrix.md`](docs/dremio-support-matrix.md)
- Editor integrations: [`docs/editor-integrations.md`](docs/editor-integrations.md)

## Contributing

- Run `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test`.
- Fixtures live under `fixtures/ansi/` and `fixtures/dremio/`; regenerate `out/` from `in/` and keep `expected/` aligned.
- See [`docs/contributing.md`](docs/contributing.md) for development workflow.
