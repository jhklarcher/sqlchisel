select 1;

select current_timestamp as now;

select
  (a + b) as sum_ab,
  ( a * (b + c) ) as nested_expr,
  price * case when country = 'SE' then 1.25 else 1.20 end as price_with_vat,
  greatest(last_login, created_at) as last_activity
from demoCatalog.sales.dim_user;
