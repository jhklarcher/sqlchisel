ALTER PIPE my_pipe SET PIPE_EXECUTION_RUNNING = TRUE;
ALTER PIPE my_pipe AS COPY INTO my_space.my_table FROM '@/files' FILE_FORMAT 'csv';
