# Dremio Dialect Notes

Use `--dialect dremio` for Dremio SQL inputs and fixtures under `fixtures/dremio/`.

## Supported Areas (Current Scope)

`sqlchisel` includes dedicated parsing/formatting support for Dremio-specific syntax beyond ANSI SQL, including:

- multi-part paths (including quoted path segments)
- versioned table references (`AT BRANCH|TAG|REF|COMMIT`, `AS OF TIMESTAMP`)
- `USE`
- reflection and acceleration commands
- pipe-style commands and other Dremio catalog/maintenance verbs
- table functions such as `TABLE(EXTERNAL_QUERY(...))`

## Formatting Expectations

- Dremio-specific recognized keywords are case-formatted according to `keyword_case`
- Version clauses are formatted as separate lines after the base `FROM` relation when present
- Quoted path segments are preserved (for example, source names with dots or dashes)
- `TABLE(EXTERNAL_QUERY(...))` and similar table functions use nested formatting so inner queries remain readable

Example:

```sql
FROM my_source.my_space.my_table
AT BRANCH my_branch
AS OF TIMESTAMP '2025-01-01 00:00:00'
```

Quoted path example:

```sql
FROM Samples."samples.dremio.com"."NYC-taxi-trips"
```

## Fixtures

When Dremio formatting behavior changes:

- update `fixtures/dremio/in/` inputs as needed
- regenerate `fixtures/dremio/out/` using `--dialect dremio`
- keep `fixtures/dremio/expected/` in sync

## Coverage Tracking

For command-level support status against the exported Dremio SQL reference, see
[`dremio-support-matrix.md`](dremio-support-matrix.md).
