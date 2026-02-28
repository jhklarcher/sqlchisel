create table if not exists arctic_catalog.analytics_space."country_metrics_daily" (
    metric_date date,
    country varchar,
    total_orders bigint,
    total_revenue double,
    avg_order_value double
);

insert into arctic_catalog.analytics_space."country_metrics_daily"
select
    cast(order_ts as date) as metric_date,
    c.country,
    count(*) as total_orders,
    sum(o.order_amount) as total_revenue,
    avg(o.order_amount) as avg_order_value
from arctic_catalog.analytics_space."fact_orders" o
join arctic_catalog.analytics_space."dim_customers" c
  on c.customer_id = o.customer_id
where order_ts >= timestamp '2024-01-01 00:00:00'
group by cast(order_ts as date), c.country
;

select * from arctic_catalog.analytics_space."country_metrics_daily"
where metric_date >= date '2024-06-01'
order by metric_date desc, country
;
