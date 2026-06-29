# dbt Syntax Support Plan

Track dbt templating support here so parser, formatter, fixtures, and docs progress stays visible in-repo.

## References

- dbt SQL models: <https://docs.getdbt.com/docs/build/sql-models>
- dbt Jinja and macros: <https://docs.getdbt.com/docs/build/jinja-macros>
- dbt Jinja functions: <https://docs.getdbt.com/reference/dbt-jinja-functions>
- dbt data tests: <https://docs.getdbt.com/docs/build/data-tests>
- Jinja template syntax: <https://jinja.palletsprojects.com/en/stable/templates/>
- Dremio dbt guide: <https://docs.dremio.com/current/data-products/deploy-with-dbt/>
- dbt-dremio walkthrough: <https://github.com/dremio/dbt-dremio/blob/main/docs/walkthrough.md>

## Status Legend

- `PASSTHROUGH`: Intentionally returned unchanged.
- `PLACEHOLDER`: Jinja/dbt syntax is protected and restored exactly while surrounding SQL can format.
- `FORMATTED`: SQL inside the syntax family is formatted where safe.
- `PARTIAL`: Some representative forms work, but known syntax remains.
- `BLOCKED`: Not supported by the current formatter architecture.

## Milestone 1: Tracker and Public Interface

- [x] Add this tracker file.
- [x] Link this tracker from `docs/README.md`.
- [x] Add `templating = "passthrough" | "dbt"` config support.
- [x] Add `--templating <passthrough|dbt>` CLI support.
- [x] Keep `templating = "passthrough"` as the default.
- [x] Include dbt SQL template extensions during directory traversal in dbt mode.
- [x] Document canonical Dremio invocation:
  `sqlchisel --dialect dremio --templating dbt --format models/my_model.sql`.

## Milestone 2: dbt/Jinja Scanner Foundation

- [x] Preserve exact contents of `{{ ... }}`, `{% ... %}`, and `{# ... #}`.
- [x] Preserve whitespace-trim forms such as `{{- ... -}}` and `{%- ... -%}`.
- [x] Support quoted strings inside template tags.
- [x] Treat `{% raw %}...{% endraw %}` as an opaque preserved block.
- [x] Protect Jinja markers inside SQL comments and string literals.
- [x] Avoid splitting SQL statements on semicolons inside dbt/Jinja tags.
- [ ] Add strict-mode diagnostics for malformed template delimiters.

## Milestone 3: Core dbt Model Syntax

- [x] Format model SQL around standalone `{{ config(...) }}` blocks.
- [x] Preserve relation-like placeholders: `ref`, versioned/two-argument `ref`, `source`, and `this`.
- [x] Preserve expression placeholders: `var`, `env_var`, `target`, `is_incremental`, `adapter`, `dispatch`, and custom macro calls.
- [x] Preserve dbt dependency comments such as `-- depends_on: {{ ref(...) }}`.
- [ ] Add broader package macro fixtures for common packages such as `dbt_utils`.

## Milestone 4: Control Flow and Generated SQL

- [x] Preserve standalone `{% if %}`, `{% elif %}`, `{% else %}`, `{% endif %}` lines.
- [x] Preserve standalone `{% for %}` and `{% endfor %}` lines.
- [x] Preserve standalone `{% set %}`, `{% endset %}`, `{% do %}`, `{% call %}`, and `{% endcall %}` lines.
- [x] Format SQL inside branch bodies when the protected SQL remains parseable.
- [ ] Add targeted handling for generated select-list comma patterns that are not parseable after placeholder preservation.

## Milestone 5: dbt Resource Coverage

- [x] Add dbt model fixtures.
- [x] Add Dremio dbt fixtures.
- [x] Add reference syntax fixtures for tests, snapshots, macros, materializations, statement blocks, hooks, operations, and package macros.
- [ ] Expand each reference syntax fixture from representative syntax to broader dbt docs coverage.
- [ ] Add optional manual smoke project for `dbt parse`/`dbt compile`.

## Milestone 6: Dremio + dbt Coverage

- [x] Add Dremio dbt fixtures for `object_storage_source`, `object_storage_path`, `dremio_space`, and `dremio_space_folder`.
- [x] Add Dremio incremental fixture with `is_incremental()` and `{{ this }}`.
- [x] Cover Dremio SQL mixed with dbt placeholders and versioned refs.
- [x] Cross-link this tracker with `docs/dremio-support-matrix.md`.
- [ ] Expand adapter-style DDL/DML fixtures as dbt-dremio coverage grows.

## Syntax Coverage Matrix

| Syntax Family | Status | Fixture Area | Notes |
| --- | --- | --- | --- |
| `{{ config(...) }}` | `PLACEHOLDER` | `fixtures/dbt/ansi`, `fixtures/dbt/dremio` | Standalone config blocks are restored exactly. |
| `ref`, versioned `ref`, `source`, `this` | `PLACEHOLDER` | `fixtures/dbt/ansi`, `fixtures/dbt/dremio` | Relation placeholders format as SQL-safe identifiers internally. |
| `var`, `env_var`, `target`, custom macros | `PLACEHOLDER` | `fixtures/dbt/ansi` | Expression placeholders are restored exactly. |
| `is_incremental()` branches | `FORMATTED` | `fixtures/dbt/dremio` | Branch SQL formats when the protected SQL remains parseable. |
| `{% if %}` / `{% for %}` control flow | `PARTIAL` | `fixtures/dbt/reference-syntax` | Standalone blocks are preserved; complex generated SQL may raw-fallback. |
| `{% macro %}` / `{% materialization %}` | `PARTIAL` | `fixtures/dbt/reference-syntax` | SQL inside parseable bodies formats; non-SQL Jinja stays opaque. |
| `{% test %}` / snapshots | `PARTIAL` | `fixtures/dbt/reference-syntax` | Representative SQL-bearing forms covered. |
| statement blocks / `run_query` | `PARTIAL` | `fixtures/dbt/reference-syntax` | Preserved and restored; SQL strings are not independently formatted. |
| dbt YAML SQL strings | `BLOCKED` | none | Out of scope for this SQL formatter. |

## Acceptance Criteria

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets --all-features -- -D warnings`
- [x] `cargo test`
- [x] Every dbt fixture formats idempotently.
- [x] Default Jinja passthrough behavior remains unchanged without `--templating dbt`.
