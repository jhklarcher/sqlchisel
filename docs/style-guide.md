# sqlchisel Style Guide (Heuristics)

This document captures current formatting heuristics and examples. It is intentionally non-stable and may evolve as formatter quality improves.

For stable guarantees, see [`format-contract.md`](format-contract.md).

## Defaults

- Line length: 100
- Indent width: 2 spaces
- Keyword case: UPPER
- Dialect: ANSI (see [`dremio.md`](dremio.md) for Dremio rules)
- Select list style: `auto` (small/medium/large tiers)

## Small / Medium / Large SELECTs

- Keep the select list inline when the estimated first line (`SELECT` + list + first `FROM`/`JOIN`) fits within `line_length`.
- Switch to per-line select items when the inline form would overflow, or when `select_list_style = per_line`.
- Single wildcard projections stay inline (`SELECT * FROM ...`) unless the `FROM` block already breaks.

Inline example:

```sql
SELECT a, b FROM t;
```

Per-line example:

```sql
SELECT
  a,
  b,
  c AS alias_c,
  CASE
    WHEN x > 0 THEN 'positive'
    ELSE 'other'
  END AS category
FROM t;
```

Hanging-line select lists are not used:

```sql
-- not used
SELECT a, b, c,
       d, e, f
FROM t;
```

## Clause Layout (ANSI)

- Clause order: `SELECT`, `FROM`, `WHERE`, `GROUP BY`, `HAVING`, `WINDOW` (if any), `QUALIFY` (if supported), `ORDER BY`, `LIMIT`/`FETCH`, `OFFSET`
- Each clause starts on its own line (except small inline `SELECT`)
- `FROM` base relation on its own line; each `JOIN` on a new line aligned with `FROM`
- Join token style is preserved (`JOIN` remains `JOIN`; `INNER JOIN` remains `INNER JOIN`)
- `ON` is indented one level under its join
- `WHERE` / `HAVING` / `QUALIFY` break logical conditions across lines
- `GROUP BY` renders one item per line when more than one expression is present
- `ORDER BY` may stay inline for short lists, but uses per-line layout for larger/longer lists
- `LIMIT` / `OFFSET` / `FETCH` stay on one line when practical

Join example:

```sql
FROM base_table AS b
INNER JOIN dim AS d
  ON d.id = b.dim_id
LEFT JOIN f
  ON f.id = b.fact_id
```

## Statement Separation

- Emit a blank line between consecutive top-level statements
- When a query has CTEs, leave a blank line between the CTE list and the main query body

## CTEs

- `WITH` on its own line
- Each CTE uses `name AS (...)`
- CTE body is indented

```sql
WITH cte AS (
  SELECT a, b FROM t
),
other AS (
  SELECT x FROM cte
)
SELECT * FROM other;
```

## Subqueries

- Keep short subqueries inline when readable
- Otherwise, use multiline parenthesized layout with indented body and aligned closing `)`
- Scalar subqueries in expressions prefer multiline layout when non-trivial

## Expressions

- `CASE` uses multiline form when not short enough to remain inline
- `WHEN` / `THEN` / `ELSE` clauses break onto separate lines in multiline form
- Nested boolean expressions break at `AND` / `OR` with indented groups

## Identifiers, Literals, Comments

- Preserve quoted identifiers as-is
- Apply keyword casing to recognized keywords only
- Preserve string literal casing and contents
- Preserve inline and block comments relative to nearby SQL where possible

## dbt/Jinja Templates

- Default templated SQL behavior is passthrough.
- With `--templating dbt`, dbt/Jinja tags are preserved exactly while surrounding SQL is formatted where safe.
- dbt templating support progress is tracked in [`dbt-support-plan.md`](dbt-support-plan.md).

## Future Tuning Areas

- Numeric thresholds for select-list layout tiers
- Additional select-list style modes
- More explicit knobs for clause wrapping thresholds
