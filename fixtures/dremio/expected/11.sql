CREATE OR REPLACE VIEW demoCatalog.sales.staging.analytics.order_customers AS
SELECT
  o.id AS order_id,
  current_timestamp AS sync_time,
  o.site_id AS site_id,
  o.site_country_id AS site_country_id,
  o.created_at AS order_created_at,
  coalesce(md5(c.email), '') AS email_hash
FROM demoCatalog.sales.staging.analytics.orders o
INNER JOIN demoCatalog.sales.staging.crm.customers c
  ON c.order_id = o.id;