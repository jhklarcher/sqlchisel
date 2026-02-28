select f.customer_id, f.order_id, f.order_ts, f.order_amount,
       c.country, c.segment,
       sum(f.order_amount) over (partition by f.customer_id order by f.order_ts
           rows between unbounded preceding and current row) as running_value,
       row_number() over (partition by f.customer_id order by f.order_ts) as order_seq
from analytics_space."fact_orders" f
join analytics_space."dim_customers" c
  on c.customer_id = f.customer_id
left join (
    select customer_id, max(order_ts) as last_order_ts
    from analytics_space."fact_orders"
    group by customer_id
) last_o
  on last_o.customer_id = f.customer_id
where f.order_ts >= timestamp '2024-01-01 00:00:00'
and f.order_amount > 0
and (c.country = 'SE' or c.country = 'NO' or c.country = 'DK')
order by f.customer_id, f.order_ts
;
