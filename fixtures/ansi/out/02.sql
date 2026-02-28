USE analytics_space."customer-metrics";

SELECT
  m.customer_id,
  m.metric_date,
  m.ltv_30d,
  m.ltv_90d,
  d.country,
  d.segment,
  CASE
    WHEN m.ltv_90d >= 1000 THEN 'VIP'
    WHEN m.ltv_90d >= 200 THEN 'REGULAR'
    ELSE 'LOW'
  END AS segment_bucket
FROM "ltv_daily" m
LEFT JOIN analytics_space."dim_customers" d
  ON d.customer_id = m.customer_id
WHERE m.metric_date BETWEEN DATE '2024-01-01' AND DATE '2024-03-31'
  AND d.opt_out_marketing = FALSE
ORDER BY m.metric_date DESC, m.customer_id;