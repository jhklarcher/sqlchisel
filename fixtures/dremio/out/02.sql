WITH base AS (
  SELECT
    vendor_id,
    pickup_datetime,
    dropoff_datetime,
    fare_amount,
    tip_amount,
    total_amount,
    passenger_count,
    CASE
      WHEN total_amount <= 0 THEN 'FREE_OR_INVALID'
      WHEN total_amount < 10 THEN 'LOW'
      WHEN total_amount < 50 THEN 'MEDIUM'
      ELSE 'HIGH'
    END AS revenue_bucket
  FROM Samples."samples.dremio.com"."NYC-taxi-trips"
),
agg AS (
  SELECT
    vendor_id,
    revenue_bucket,
    COUNT(*) AS num_trips,
    SUM(total_amount) AS total_revenue,
    AVG(total_amount) AS avg_revenue
  FROM base
  GROUP BY
    vendor_id,
    revenue_bucket
)

SELECT
  a.vendor_id,
  a.revenue_bucket,
  a.num_trips,
  a.total_revenue,
  a.avg_revenue,
  RANK() OVER (
    PARTITION BY a.revenue_bucket
    ORDER BY a.total_revenue DESC
  ) AS revenue_rank
FROM agg a
WHERE a.num_trips > 100
ORDER BY a.revenue_bucket, a.revenue_rank;