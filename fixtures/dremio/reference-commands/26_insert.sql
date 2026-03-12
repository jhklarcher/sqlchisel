INSERT INTO my_space.my_table (id, amount) VALUES (1, 10.5), (2, 20.0);
INSERT INTO my_space.my_table SELECT id, amount FROM my_space.source_table;
INSERT INTO my_space.my_table SELECT id, amount FROM my_space.source_table AT COMMIT 'a1b2c3d4';
