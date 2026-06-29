{% materialization table, adapter="dremio" %}
{% set target_relation = this %}
{% call statement("main") %}
create table {{ target_relation }} as select * from {{ ref("orders") }}
{% endcall %}
{% do adapter.commit() %}
{% endmaterialization %}
