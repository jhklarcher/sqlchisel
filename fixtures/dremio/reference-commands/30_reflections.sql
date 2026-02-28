CREATE REFLECTION my_ref USING TABLE my_space.my_table;
REFRESH ACCELERATION my_ref WITH (refresh = 'auto');
