select id, country
from dim_customers
where active = true

union all

select customer_id, country_code
from fact_orders
where total_amount > 0;

with base as (
  select id from dim_customers where active = true
),
archived as (
  select id from dim_customers_archive
)
select id from base
intersect
select id from archived;
