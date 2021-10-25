drop table if exists queues_answers;
drop table if exists answers_queues;
drop table if exists answers;
drop table if exists answer_states;
drop table if exists queues;
drop table if exists temp_questions;

-- Drop author id and updated_at
create table temp_questions (
  id text not null primary key,
  link text,
  link_logo text,
  created_at text not null
);
insert into temp_questions select id, link, link_logo, created_at from questions;
drop table questions;
alter table temp_questions rename to questions;

drop table if exists users;
