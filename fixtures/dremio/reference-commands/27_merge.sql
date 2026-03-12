MERGE INTO my_space.target t USING my_space.source s ON t.id = s.id WHEN MATCHED THEN UPDATE SET amount = s.amount WHEN NOT MATCHED THEN INSERT (id, amount) VALUES (s.id, s.amount);
MERGE INTO my_space.target t USING (SELECT id, amount FROM my_space.source) s ON t.id = s.id WHEN MATCHED THEN UPDATE SET amount = s.amount;
