create   space demo_space;
ALTER   SPACE demo_space   RENAME TO demo_space_v2 ;
drop   space  demo_space_v2;

create source demo_source type s3 with ( access_key = 'abc', secret_key='xyz');
DROP  SOURCE demo_source ;
