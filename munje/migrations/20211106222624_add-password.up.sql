alter table users add column hashed_password text;
update users set hashed_password = '*';
alter table users alter column hashed_password set not null;
