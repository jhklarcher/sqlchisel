DROP TABLE IF EXISTS demoCatalog.app.staging.reports.{{ ti.xcom_pull(task_ids='determine_target_table') }};

CREATE TABLE demoCatalog.app.staging.reports.{{ ti.xcom_pull(task_ids='determine_target_table') }} AS
SELECT /*+ no_reflections */
    *
FROM external_cluster.app.raw_segments;
