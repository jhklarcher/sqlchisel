{{ config(materialized="incremental", object_storage_source="lake", object_storage_path="/dbt", dremio_space="analytics", dremio_space_folder="marts") }}

select * from {{ source("raw", "orders") }} at branch {{ var("nessie_branch") }}
{% if is_incremental() %}
where updated_at > (select max(updated_at) from {{ this }})
{% endif %}
