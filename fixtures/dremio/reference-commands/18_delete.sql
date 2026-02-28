DELETE FROM my_space.my_table WHERE id = 1;
DELETE FROM my_space.my_table WHERE id IN (SELECT id FROM my_space.ids_to_delete);
