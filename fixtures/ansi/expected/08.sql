SELECT
  *
FROM {{ ref("orders_table") }}
WHERE order_date >= {{ start_date }}
  {% if include_cancelled %}
  AND status <> 'CANCELLED'
  {% endif %}
;
