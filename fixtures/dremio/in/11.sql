create or replace view demoCatalog.sales.staging.analytics.order_customers as
select
  o.id as order_id,
  current_timestamp as sync_time,
  o.site_id as site_id,
  o.site_country_id as site_country_id,
  o.created_at as order_created_at,
  coalesce(md5(c.email), '') as email_hash
from demoCatalog.sales.staging.analytics.orders o
join demoCatalog.sales.staging.crm.customers c
  on c.order_id = o.id;
