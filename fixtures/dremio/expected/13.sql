SELECT 1;

SELECT current_timestamp AS now;

SELECT
  (a + b) AS sum_ab,
  (a * (b + c)) AS nested_expr,
  price * CASE
    WHEN country = 'SE' THEN 1.25
    ELSE 1.20
  END AS price_with_vat,
  greatest(last_login, created_at) AS last_activity
FROM demoCatalog.sales.dim_user;