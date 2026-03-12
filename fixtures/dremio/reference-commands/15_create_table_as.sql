CREATE TABLE my_space.ctas_table AS SELECT * FROM my_space.my_table;
CREATE TABLE my_space.ctas_partitioned PARTITION BY (MONTH(created_at)) AS SELECT id, created_at FROM my_space.my_table;
