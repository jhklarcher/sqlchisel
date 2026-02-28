SELECT id, country FROM dim_customers WHERE active = true UNION ALL SELECT customer_id, country_code FROM fact_orders WHERE total_amount > 0;

WITH base AS (
  SELECT id FROM dim_customers
  WHERE active = TRUE
),
archived AS (
  SELECT id FROM dim_customers_archive
)

SELECT id FROM base INTERSECT SELECT id FROM archived;