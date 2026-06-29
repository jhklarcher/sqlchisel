# Dremio SQL Reference Coverage Matrix

This file tracks `sqlchisel` support against the Dremio SQL command pages listed in [`sql-reference-found.md`](../sql-reference-found.md).

Evaluated on: 2026-02-28  
Reference export timestamp: 2026-02-28T14:20:54.462923+00:00

## Status Legend

- `NATIVE`: Custom Dremio parser + formatter path (`DremioCommand`).
- `AST`: Parsed as SQL AST and formatted through generic SQL statement handling.
- `PARTIAL`: Some forms work, but important syntax from Dremio reference is missing or fallback-ish.
- `UNSUPPORTED`: Representative command syntax fails strict parse today.
- `INDEX`: Reference index/overview page, not a direct SQL command syntax page.

## Where Formatter Behavior Is Tracked

- Stable guarantees: [`docs/format-contract.md`](format-contract.md)
- Formatting heuristics: [`docs/style-guide.md`](style-guide.md)
- Dremio notes (high-level): [`docs/dremio.md`](dremio.md)
- Dremio command parser: [`src/parser.rs`](../src/parser.rs) (`parse_dremio_command`)
- SQL statement formatter dispatch: [`src/format/sql/mod.rs`](../src/format/sql/mod.rs) (`format_statement`)
- Dremio command formatter: [`src/format/sql/dremio.rs`](../src/format/sql/dremio.rs)
- Regression fixtures: `fixtures/dremio/{in,expected,out}`
- dbt/Jinja templating on Dremio: [`docs/dbt-support-plan.md`](dbt-support-plan.md)

## Coverage Matrix (57 Dremio Command Pages)

| # | Dremio Reference Page | Status | Fixture | Notes |
| --- | --- | --- | --- | --- |
| 1 | ALTER BRANCH | NATIVE | `fixtures/dremio/reference-commands/01_alter_branch.sql` | `BranchTag` command path |
| 2 | ALTER FOLDER | NATIVE | `fixtures/dremio/reference-commands/02_alter_folder.sql` | Generic Dremio command path |
| 3 | ALTER PIPE Enterprise | NATIVE | `fixtures/dremio/reference-commands/03_alter_pipe.sql` | `Pipe` command path |
| 4 | ALTER SOURCE | NATIVE | `fixtures/dremio/reference-commands/04_alter_source.sql` | Generic Dremio command path |
| 5 | ALTER SPACE | NATIVE | `fixtures/dremio/reference-commands/05_alter_space.sql` | Generic Dremio command path |
| 6 | ALTER TABLE | NATIVE | `fixtures/dremio/reference-commands/06_alter_table.sql` | Reflection-specific + generic ALTER TABLE path |
| 7 | ALTER TAG | NATIVE | `fixtures/dremio/reference-commands/07_alter_tag.sql` | `BranchTag` command path |
| 8 | ALTER VIEW | NATIVE | `fixtures/dremio/reference-commands/08_alter_view.sql` | Generic Dremio command path |
| 9 | ANALYZE TABLE | NATIVE | `fixtures/dremio/reference-commands/09_analyze_table.sql` | `AnalyzeTable` command path |
| 10 | COPY INTO | NATIVE | `fixtures/dremio/reference-commands/10_copy_into.sql` | Supports both `COPY INTO` and `COPY INTO TABLE` |
| 11 | CREATE BRANCH | NATIVE | `fixtures/dremio/reference-commands/11_create_branch.sql` | `BranchTag` command path |
| 12 | CREATE FOLDER | NATIVE | `fixtures/dremio/reference-commands/12_create_folder.sql` | `CreateFolder` command path |
| 13 | CREATE PIPE Enterprise | NATIVE | `fixtures/dremio/reference-commands/13_create_pipe.sql` | `Pipe` command path |
| 14 | CREATE TABLE | AST | `fixtures/dremio/reference-commands/14_create_table.sql` | SQL AST path |
| 15 | CREATE TABLE AS | AST | `fixtures/dremio/reference-commands/15_create_table_as.sql` | SQL AST path + versioned source ref variant |
| 16 | CREATE TAG | NATIVE | `fixtures/dremio/reference-commands/16_create_tag.sql` | `BranchTag` command path |
| 17 | CREATE VIEW | AST | `fixtures/dremio/reference-commands/17_create_view.sql` | SQL AST path + versioned source ref variant |
| 18 | DELETE | AST | `fixtures/dremio/reference-commands/18_delete.sql` | SQL AST path |
| 19 | DESCRIBE PIPE Enterprise | NATIVE | `fixtures/dremio/reference-commands/19_describe_pipe.sql` | `Pipe` command path |
| 20 | DROP | AST | `fixtures/dremio/reference-commands/20_drop.sql` | SQL AST path |
| 21 | DROP BRANCH | NATIVE | `fixtures/dremio/reference-commands/21_drop_branch.sql` | `BranchTag` command path |
| 22 | DROP PIPE Enterprise | NATIVE | `fixtures/dremio/reference-commands/22_drop_pipe.sql` | `Pipe` command path |
| 23 | DROP TAG | NATIVE | `fixtures/dremio/reference-commands/23_drop_tag.sql` | `BranchTag` command path |
| 24 | DROP VIEW | AST | `fixtures/dremio/reference-commands/24_drop_view.sql` | SQL AST path |
| 25 | GRANT/REVOKE Enterprise | NATIVE | `fixtures/dremio/reference-commands/25_grant_revoke.sql` | Generic Dremio command path |
| 26 | INSERT | AST | `fixtures/dremio/reference-commands/26_insert.sql` | SQL AST + insert formatting path + versioned source ref variant |
| 27 | MERGE | AST | `fixtures/dremio/reference-commands/27_merge.sql` | SQL AST path |
| 28 | MERGE BRANCH | NATIVE | `fixtures/dremio/reference-commands/28_merge_branch.sql` | `BranchTag` command path |
| 29 | OPTIMIZE TABLE | NATIVE | `fixtures/dremio/reference-commands/29_optimize_table.sql` | `TableMaintenance` command path |
| 30 | Reflections | NATIVE | `fixtures/dremio/reference-commands/30_reflections.sql` | `Reflection` / `Acceleration*` command paths |
| 31 | RESET QUEUE | NATIVE | `fixtures/dremio/reference-commands/31_reset_queue.sql` | `QueueTag` command path |
| 32 | RESET TAG | NATIVE | `fixtures/dremio/reference-commands/32_reset_tag.sql` | `QueueTag` command path |
| 33 | Role Enterprise | NATIVE | `fixtures/dremio/reference-commands/33_role_enterprise.sql` | Generic Dremio role command path |
| 34 | ROLLBACK | NATIVE | `fixtures/dremio/reference-commands/34_rollback.sql` | `TableMaintenance` command path |
| 35 | Row-Access & Column-Masking | NATIVE | `fixtures/dremio/reference-commands/35_row_access_column_masking.sql` | `RowColumnPolicies` + generic ALTER TABLE path |
| 36 | SELECT | AST | `fixtures/dremio/reference-commands/36_select.sql` | SQL AST path with guarded query formatting |
| 37 | SET QUEUE | NATIVE | `fixtures/dremio/reference-commands/37_set_queue.sql` | `QueueTag` command path |
| 38 | SET TAG | NATIVE | `fixtures/dremio/reference-commands/38_set_tag.sql` | `QueueTag` command path |
| 39 | SHOW BRANCHES | NATIVE | `fixtures/dremio/reference-commands/39_show_branches.sql` | `Show` command path |
| 40 | SHOW CREATE TABLE | AST | `fixtures/dremio/reference-commands/40_show_create_table.sql` | SQL AST path |
| 41 | SHOW CREATE VIEW | AST | `fixtures/dremio/reference-commands/41_show_create_view.sql` | SQL AST path |
| 42 | SHOW LOGS | NATIVE | `fixtures/dremio/reference-commands/42_show_logs.sql` | `Show` command path |
| 43 | SHOW TAGS | NATIVE | `fixtures/dremio/reference-commands/43_show_tags.sql` | `Show` command path |
| 44 | SHOW TBLPROPERTIES | NATIVE | `fixtures/dremio/reference-commands/44_show_tblproperties.sql` | Accepts both short and long property forms |
| 45 | Source SQL Statements | INDEX | `fixtures/dremio/reference-commands/45_source_sql_statements.sql` | Index/overview page |
| 46 | SQL Commands for Apache Iceberg Tables | INDEX | `fixtures/dremio/reference-commands/46_sql_commands_apache_iceberg_tables.sql` | Index/overview page |
| 47 | SQL Commands for Nessie | INDEX | `fixtures/dremio/reference-commands/47_sql_commands_nessie.sql` | Index/overview page |
| 48 | SQL Commands Reference | INDEX | `fixtures/dremio/reference-commands/48_sql_commands_reference.sql` | Index/overview page |
| 49 | Table SQL Statements | INDEX | `fixtures/dremio/reference-commands/49_table_sql_statements.sql` | Index/overview page |
| 50 | TRUNCATE | AST | `fixtures/dremio/reference-commands/50_truncate.sql` | SQL AST path |
| 51 | UPDATE | AST | `fixtures/dremio/reference-commands/51_update.sql` | SQL AST path |
| 52 | USE | NATIVE | `fixtures/dremio/reference-commands/52_use.sql` | `Use` command path |
| 53 | User Enterprise | NATIVE | `fixtures/dremio/reference-commands/53_user_enterprise.sql` | Generic Dremio user command path |
| 54 | User-Defined Functions | NATIVE | `fixtures/dremio/reference-commands/54_user_defined_functions.sql` | Generic Dremio function command path |
| 55 | VACUUM CATALOG | NATIVE | `fixtures/dremio/reference-commands/55_vacuum_catalog.sql` | `VacuumCatalog` command path |
| 56 | VACUUM TABLE | NATIVE | `fixtures/dremio/reference-commands/56_vacuum_table.sql` | `TableMaintenance` command path |
| 57 | WITH | AST | `fixtures/dremio/reference-commands/57_with.sql` | SQL AST path |

## Snapshot Summary

- `NATIVE`: 38
- `AST`: 14
- `PARTIAL`: 0
- `UNSUPPORTED`: 0
- `INDEX`: 5

## Evaluation Notes

- Canonical command corpus: `fixtures/dremio/reference-commands/` with 57 fixture files.
- Versioned source refs are covered in canonical `CREATE TABLE AS`, `CREATE VIEW`, and `INSERT` fixtures.
- Parser guard: strict-mode test covers all 57 fixture files.
- Formatter guard: idempotence + keyword-case matrix (upper/lower) covers all 57 fixture files.
- Statement-level semantic-preservation tests assert version-clause retention for CTAS/CREATE VIEW/INSERT in parser + formatter paths.
- `sqlparser` has been upgraded to `0.61.0` and all parser/formatter tests pass with the upgraded AST APIs.

## Backlog (Priority Order)

1. Expand canonical fixtures incrementally from representative statements to broader per-page syntax variants.
2. Keep matrix + fixtures + parser/formatter tests in lockstep for any future Dremio syntax change.
