SELECT *
FROM dim_customers c
INNER JOIN fact_orders o USING customer_id
CROSS JOIN dim_dates d
NATURAL LEFT JOIN dim_flags f;