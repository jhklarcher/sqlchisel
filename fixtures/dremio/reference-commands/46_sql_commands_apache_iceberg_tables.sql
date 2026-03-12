OPTIMIZE TABLE my_space.iceberg_table;
VACUUM TABLE my_space.iceberg_table EXPIRE SNAPSHOTS RETAIN_LAST = 5;
