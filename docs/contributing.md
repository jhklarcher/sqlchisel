# Contributing to sqlchisel

This guide covers day-to-day development practices for `sqlchisel`.

## Setup

- Install Rust (stable) and ensure `cargo` is on your PATH.
- Clone the repo and run `cargo build` once to fetch dependencies.

## Everyday Commands

- Format: `cargo fmt`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`
- Tests: `cargo test`
- CI equivalent locally: run all three commands above.

## Development Workflow

- Align docs when behavior changes:
  - `README.md` (user-facing quickstart)
  - `docs/format-contract.md` (stable behavior)
  - `docs/style-guide.md` (heuristics/taste)
  - `docs/dremio.md` (dialect-specific behavior)
- Keep `docs/repo-cleanup-checklist.md` updated while the docs/module split effort is in progress.
- Prefer using `--format`, `--check`, or `--write` locally to mirror user workflows.
- For Dremio work, add parser + formatter coverage and tests together.

## Code Style

- Rust edition 2021.
- Avoid non-ASCII unless the file already uses it or it is required.
- Keep code comments concise; favor clear code and targeted comments for tricky bits.

## Pull Requests

- Ensure `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test` all pass.
- Include tests for new behavior (parser/formatter).
- Update docs as needed (`README.md` and relevant files in `docs/`).
