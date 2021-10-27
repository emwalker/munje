delete from answers;
delete from queues;
delete from questions;
drop table questions;

create table questions (
  id text not null primary key,
  author_id text not null,
  title text not null,
  text text not null,
  link text,
  link_logo text,
  created_at text not null,
  updated_at text not null,
  foreign key (author_id) references users(id)
);
