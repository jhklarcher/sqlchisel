with base as (
    select
        vendor_id,
        pickup_datetime,
        dropoff_datetime,
        fare_amount,
        tip_amount,
        total_amount,
        passenger_count,
        case
            when total_amount <= 0 then 'FREE_OR_INVALID'
            when total_amount < 10 then 'LOW'
            when total_amount < 50 then 'MEDIUM'
            else 'HIGH'
        end as revenue_bucket
    from Samples."samples.dremio.com"."NYC-taxi-trips"
),
agg as (
    select
        vendor_id,
        revenue_bucket,
        COUNT(*) as num_trips,
        SUM(total_amount) as total_revenue,
        avg(total_amount) as avg_revenue
    from base
    group by vendor_id, revenue_bucket
)
select a.vendor_id, a.revenue_bucket, a.num_trips, a.total_revenue, a.avg_revenue,
       RANK() OVER (PARTITION BY a.revenue_bucket order by a.total_revenue desc) as revenue_rank
from agg a
where a.num_trips> 100
order by a.revenue_bucket, a.revenue_rank
;
