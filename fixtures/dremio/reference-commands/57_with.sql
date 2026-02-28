WITH cte AS (SELECT id FROM my_space.my_table) SELECT * FROM cte;
WITH a AS (SELECT 1 AS id), b AS (SELECT id FROM a) SELECT id FROM b;
