CREATE PIPE IF NOT EXISTS my_pipe NOTIFICATION_PROVIDER AWS_SQS NOTIFICATION_QUEUE_REFERENCE 'arn:aws:sqs:::queue' AS COPY INTO my_space.my_table FROM '@/files' FILE_FORMAT 'csv';
CREATE PIPE my_pipe2 AS COPY INTO my_space.my_table FROM '@/files' FILE_FORMAT 'parquet';
