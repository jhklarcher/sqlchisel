CREATE OR REPLACE VIEW demoCatalog.sales.staging.geo.continent AS
SELECT * FROM source_cluster.geo."continent";

CREATE VIEW demoCatalog.sales.staging.geo.country AS
SELECT code, name FROM source_cluster.geo."country";