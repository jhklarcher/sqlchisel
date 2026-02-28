CREATE TABLE arctic_catalog.analytics_space."daily_revenue_by_country" AS
SELECT
  CAST(order_ts AS DATE) AS order_date,
  c.country,
  SUM(o.order_amount) AS total_revenue,
  COUNT(*) AS num_orders,
  AVG(o.order_amount) AS avg_order_value
FROM arctic_catalog.analytics_space."fact_orders" o
INNER JOIN arctic_catalog.analytics_space."dim_customers" c
  ON c.customer_id = o.customer_id
WHERE order_ts >= TIMESTAMP '2024-01-01 00:00:00'
GROUP BY
  CAST(order_ts AS DATE),
  c.country
HAVING SUM(o.order_amount) > 1000;