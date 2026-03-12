SELECT *
FROM demoCatalog.sales."orders_history"
AT TAG release_2024_01;

SELECT *
FROM demoCatalog.sales."orders_history"
AT REF "refs/heads/main";

CREATE TABLE demoCatalog.sales.snapshot_orders AS
SELECT *
FROM demoCatalog.sales."orders_history"
AT COMMIT 'a1b2c3d4';