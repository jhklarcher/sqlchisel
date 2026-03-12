ALTER TABLE my_space.my_table ADD COLUMNS (new_col INT);
ALTER TABLE my_space.my_table CREATE RAW REFLECTION my_ref USING DISPLAY (id, new_col);
