COPY INTO my_space.my_table FROM '@/files' FILE_FORMAT 'csv';
COPY INTO TABLE my_space.my_table FROM '@/files' FILE_FORMAT 'json';
