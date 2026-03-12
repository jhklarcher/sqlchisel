CREATE OR REPLACE VIEW my_space.my_view AS SELECT id FROM my_space.my_table;
CREATE VIEW my_space.my_view2 AS SELECT id, SUM(amount) AS total_amount FROM my_space.my_table GROUP BY id;
CREATE VIEW my_space.my_view_from_branch AS SELECT id FROM my_space.my_table AT BRANCH release AS OF TIMESTAMP '2025-01-01 00:00:00';
