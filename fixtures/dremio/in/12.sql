select * from demoCatalog.sales."orders_history" at tag release_2024_01;

select * from demoCatalog.sales."orders_history" at ref "refs/heads/main";

create table demoCatalog.sales.snapshot_orders as
select *
from demoCatalog.sales."orders_history" at commit 'a1b2c3d4';
