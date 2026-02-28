SELECT id, amount FROM my_space.my_table WHERE amount > 0 ORDER BY amount DESC LIMIT 10;
WITH ranked AS (SELECT id, amount, ROW_NUMBER() OVER (PARTITION BY id ORDER BY amount DESC) AS rn FROM my_space.my_table) SELECT id, amount FROM ranked QUALIFY rn = 1;
