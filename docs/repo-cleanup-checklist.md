# Repo Cleanup Checklist

Track the docs split and formatter module refactor here so progress is visible in-repo.

## Phase 1: Docs and README cleanup

- [x] Create `docs/` and a docs index (`docs/README.md`)
- [x] Split user docs into focused files (`cli-reference`, `config`, `format-contract`, `style-guide`, `dremio`, `editor-integrations`)
- [x] Add `docs/contributing.md` and point contributor links to `docs/`
- [x] Slim `README.md` to install + core workflows + highlights + docs index
- [x] Retire root `STYLE.md` in favor of split docs in `docs/`
- [x] Remove root `CONTRIBUTING.md` after moving content into `docs/contributing.md`
- [x] Remove user-facing `AGENTS.md` references from `README.md`
- [x] Remove root `AGENTS.md`
- [x] Remove `.project/` from repo root

## Phase 2: Behavior contract hardening

- [x] Expand `docs/format-contract.md` with explicit exit-code/error behavior examples
- [x] Add fixture-backed examples for contract rules (semicolon preservation, statement spacing, raw fallback)
- [x] Mark any unstable areas explicitly instead of leaving draft prose in stable docs

## Phase 3: Formatter module refactor (no behavior changes)

- [x] Convert `src/format/sql.rs` into `src/format/sql/` with `mod.rs` facade
- [x] Extract `raw.rs` (raw token formatting fallback)
- [x] Extract `literals.rs` (string literal placeholder/restore)
- [x] Extract `jinja.rs` (Jinja detection/preserve/restore)
- [x] Extract `layout.rs` (select layout heuristics and estimators)
- [x] Extract `dremio.rs` (Dremio command formatting)
- [x] Extract `query.rs`, `select.rs`, and `from_join.rs` modules
- [x] Extract `expr.rs` and `case.rs` modules
- [x] Extract `comments.rs` last

## Phase 4: Validation after each refactor step

- [x] Run `cargo fmt`
- [x] Run `cargo clippy --all-targets --all-features -- -D warnings`
- [x] Run `cargo test`
- [x] Verify fixtures remain unchanged unless intentionally updated
