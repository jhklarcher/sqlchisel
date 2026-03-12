create table a as select * from s.t at tag v1;
create view v as select * from s.t at branch release as of timestamp '2025-01-01 00:00:00';
insert into a select * from s.t at commit 'a1b2c3d4';
select * from s.t at ref "refs/heads/main";
