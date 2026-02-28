select
  *
from dim_customers c
join fact_orders o using (customer_id)
cross join dim_dates d
natural left join dim_flags f;
