CREATE OR REPLACE VIEW my_space.my_view AS SELECT id FROM my_space.my_table;
CREATE VIEW my_space.my_view2 AS SELECT id, SUM(amount) AS total_amount FROM my_space.my_table GROUP BY id;
