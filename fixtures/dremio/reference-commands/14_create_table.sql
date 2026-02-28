CREATE TABLE IF NOT EXISTS my_space.my_table (id INT, created_at TIMESTAMP);
CREATE TABLE my_space.partitioned (id INT, event_ts TIMESTAMP) PARTITION BY (MONTH(event_ts));
