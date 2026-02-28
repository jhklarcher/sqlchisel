UPDATE my_space.my_table SET amount = amount + 1 WHERE id = 1;
UPDATE my_space.my_table AS t SET amount = s.amount FROM my_space.source_table AS s WHERE t.id = s.id;
