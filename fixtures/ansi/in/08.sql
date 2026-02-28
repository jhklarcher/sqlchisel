select
  *
from {{ ref("orders_table") }}
where order_date >= {{ start_date }}
  {% if include_cancelled %}
  and status <> 'CANCELLED'
  {% endif %}
;
