{{ config(materialized="table") }}

select id, {{ var("country_expression", "country") }} as country, amount from {{ ref("orders") }} where created_at >= {{ var("start_date") }}
-- depends_on: {{ ref("dim_dates") }}
