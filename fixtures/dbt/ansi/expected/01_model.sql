{{ config(materialized="table") }}
SELECT id, {{ var("country_expression", "country") }} AS country, amount FROM {{ ref("orders") }}
WHERE created_at >= {{ var("start_date") }}
-- depends_on: {{ ref("dim_dates") }}
