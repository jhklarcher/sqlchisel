SELECT
  f.customer_id,
  f.order_id,
  f.order_ts,
  f.order_amount,
  c.country,
  c.segment,
  SUM(f.order_amount) OVER (
    PARTITION BY f.customer_id
    ORDER BY f.order_ts
    ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
  ) AS running_value,
  ROW_NUMBER() OVER (
    PARTITION BY f.customer_id
    ORDER BY f.order_ts
  ) AS order_seq
FROM analytics_space."fact_orders" f
INNER JOIN analytics_space."dim_customers" c
  ON c.customer_id = f.customer_id
LEFT JOIN (
  SELECT
    customer_id,
    MAX(order_ts) AS last_order_ts
  FROM analytics_space."fact_orders"
  GROUP BY customer_id
) last_o
  ON last_o.customer_id = f.customer_id
WHERE f.order_ts >= TIMESTAMP '2024-01-01 00:00:00'
  AND f.order_amount > 0
  AND (
    c.country = 'SE'
    OR c.country = 'NO'
    OR c.country = 'DK'
  )
ORDER BY f.customer_id, f.order_ts;