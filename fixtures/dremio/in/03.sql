CREATE TABLE demoCatalog.reporting."tables".orders_partitioned AS (
SELECT 
    o.id AS order_id,
    CURRENT_TIMESTAMP AS sync_time,
    o.order_number AS order_number,
    co.site_id AS site_id,
    s.brand_id AS brand_id,
    s.country_id AS site_country_id,
    o.created_at AS created_at,
    COALESCE(dist.revenue_share, 0) AS distribution_amount,
    CASE WHEN o.change_time IS NULL THEN 0 ELSE 1 END AS is_changed,
    CASE WHEN o.cancel_time IS NULL THEN 0 ELSE 1 END AS is_canceled
FROM demoCatalog.reporting."tables"."orders" o
JOIN demoCatalog.reporting."tables"."cart_orders" co ON co.order_id = o.id
JOIN demoCatalog.reporting."tables"."order_revenues" orev ON orev.order_id = o.id
JOIN demoCatalog.reporting."tables"."sites" s ON s.id = co.site_id
LEFT JOIN (
    SELECT i.order_id, SUM(i.revenue) AS revenue_share
    FROM demoCatalog.reporting."tables"."order_revenue_items" i
    JOIN demoCatalog.reporting."tables"."product_types" pt ON i.product_type_id = pt.id
    WHERE pt.revenue_group_id = 17
    GROUP BY i.order_id
) dist ON dist.order_id = o.id
WHERE NOT EXISTS (
      SELECT 1
      FROM demoCatalog.reporting."tables"."order_items" oi
      JOIN demoCatalog.reporting."tables"."customer_items" ci ON ci.id = oi.customer_item_id
      WHERE oi.order_id = o.id AND ci.product_type_id = 620
  )
  AND (o.is_test_order = FALSE)
);
