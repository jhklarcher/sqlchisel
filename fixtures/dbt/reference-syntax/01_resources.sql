{% snapshot orders_snapshot %}
{{ config(target_schema="snapshots", unique_key="id", strategy="timestamp", updated_at="updated_at") }}
select id, updated_at from {{ source("raw", "orders") }}
{% endsnapshot %}

{% test accepted_status(model, column_name) %}
select * from {{ model }} where {{ column_name }} not in ("placed", "shipped")
{% endtest %}

{% macro cents_to_dollars(column_name, scale=2) %}
({{ column_name }} / 100)::numeric(16, {{ scale }})
{% endmacro %}
