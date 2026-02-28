SELECT
  o.order_id,
  o.customer_id,
  o.order_ts,
  o.status,
  i.sku,
  i.quantity,
  i.extended_price,
  CASE
    WHEN o.status IN ('CANCELLED', 'RETURNED') THEN 1
    ELSE 0
  END AS is_problem
FROM arctic_catalog.analytics_space.orders_history o
AT BRANCH feature_campaign_x
AS OF TIMESTAMP '2024-10-01 12:00:00'
INNER JOIN arctic_catalog.analytics_space.order_items_history i
  ON i.order_id = o.order_id
  AND i.valid_from <= o.order_ts
  AND (
    i.valid_to IS NULL
    OR i.valid_to > o.order_ts
  )
WHERE o.order_ts BETWEEN TIMESTAMP '2024-09-01 00:00:00' AND TIMESTAMP '2024-09-30 23:59:59'
  AND o.channel IN ('web', 'mobile')
ORDER BY o.order_ts DESC, o.order_id;