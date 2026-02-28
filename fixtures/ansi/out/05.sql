CREATE space demo_space;

ALTER SPACE demo_space RENAME TO demo_space_v2;

DROP space demo_space_v2;

CREATE SOURCE demo_source TYPE s3
WITH (access_key = 'abc', secret_key = 'xyz');

DROP SOURCE demo_source;