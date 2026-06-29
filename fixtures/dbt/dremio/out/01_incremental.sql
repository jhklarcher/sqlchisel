{{ config(materialized="incremental", object_storage_source="lake", object_storage_path="/dbt", dremio_space="analytics", dremio_space_folder="marts") }}
SELECT *
FROM {{ source("raw", "orders") }}
AT BRANCH {{ var("nessie_branch") }}
{% if is_incremental() %}
WHERE updated_at > (
    SELECT MAX(updated_at) FROM {{ this }}
  )
{% endif %}
