create or replace view demoCatalog.sales.staging.geo.continent as
select * from source_cluster.geo."continent";

create view demoCatalog.sales.staging.geo.country as
select code, name from source_cluster.geo."country";
