create   table  arctic_catalog.analytics_space."daily_revenue_by_country"  as
select
    cast(order_ts as date) as order_date ,
    c.country ,
    sum(o.order_amount) as total_revenue ,
    count(*) as num_orders ,
    avg(o.order_amount) as avg_order_value
from arctic_catalog.analytics_space."fact_orders"    o
join arctic_catalog.analytics_space."dim_customers" c
on c.customer_id=o.customer_id
where order_ts >= timestamp '2024-01-01 00:00:00'
group by cast(order_ts as date), c.country
having sum(o.order_amount) > 1000
;
