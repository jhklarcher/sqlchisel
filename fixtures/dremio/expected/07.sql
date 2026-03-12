CREATE TABLE IF NOT EXISTS arctic_catalog.analytics_space."country_metrics_daily" (
  metric_date DATE,
  country VARCHAR,
  total_orders BIGINT,
  total_revenue DOUBLE,
  avg_order_value DOUBLE
);

INSERT INTO arctic_catalog.analytics_space."country_metrics_daily"
SELECT
  CAST(order_ts AS DATE) AS metric_date,
  c.country,
  COUNT(*) AS total_orders,
  SUM(o.order_amount) AS total_revenue,
  AVG(o.order_amount) AS avg_order_value
FROM arctic_catalog.analytics_space."fact_orders" o
JOIN arctic_catalog.analytics_space."dim_customers" c
  ON c.customer_id = o.customer_id
WHERE order_ts >= TIMESTAMP '2024-01-01 00:00:00'
GROUP BY
  CAST(order_ts AS DATE),
  c.country;

SELECT * FROM arctic_catalog.analytics_space."country_metrics_daily"
WHERE metric_date >= DATE '2024-06-01'
ORDER BY metric_date DESC, country;