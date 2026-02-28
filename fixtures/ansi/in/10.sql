-- Refresh step
{{ ref("refresh_dim_customers") }}

-- Main query
SELECT
    *
FROM {{ ref("orders") }}  -- main fact table
WHERE created_at >= {{ start_date }}  -- dynamic lower bound
{# Jinja-only comment #}
;
