create or replace function set_updated_at_column()
returns trigger as $$
begin
    new.updated_at = now();
    return new;
end;
$$ language 'plpgsql';

create table users (
  created_at timestamp with time zone not null default now(),
  handle varchar(30) unique not null,
  id bigserial primary key,
  updated_at timestamp with time zone not null default now()
);

create trigger trigger_users_set_updated_at_column before update on users
  for each row execute procedure set_updated_at_column();

create table questions (
  author_id bigint not null references users,
  created_at timestamp with time zone not null default now(),
  external_id varchar(12) unique not null,
  id bigserial primary key,
  link text,
  link_logo text,
  text text not null,
  title text not null,
  updated_at timestamp with time zone not null default now()
);

create trigger trigger_questions_set_updated_at_column before update on questions
  for each row execute procedure set_updated_at_column();

create table queues (
  created_at timestamp with time zone not null default now(),
  external_id varchar(12) unique not null,
  id bigserial primary key,
  starting_question_id bigint not null references questions,
  updated_at timestamp with time zone not null default now(),
  title text not null,
  description text,
  user_id bigint not null references users,
  unique (user_id, starting_question_id)
);

create trigger trigger_queues_set_updated_at_column before update on queues
  for each row execute procedure set_updated_at_column();

create table answer_states (
  type varchar(20) not null primary key,
  "order" integer unique not null
);

insert into answer_states (type, "order")
  values ('correct', 0), ('incorrect', 1), ('unsure', 2);

-- Answers are immutable
create table answers (
  answered_at timestamp with time zone not null default now(),
  consecutive_correct int not null,
  external_id varchar(12) unique not null,
  id bigserial primary key,
  question_id bigint not null references questions,
  queue_id bigint not null references queues,
  state varchar(20) not null references answer_states,
  user_id bigint not null references users
);

create table last_answers (
  answer_answered_at timestamp with time zone not null default now(),
  answer_consecutive_correct int not null,
  answer_id bigint not null references answers,
  answer_state varchar(20) not null references answer_states,
  created_at timestamp with time zone not null default now(),
  id bigserial primary key not null,
  question_id bigint not null references questions,
  queue_id bigint not null references queues,
  updated_at timestamp with time zone not null default now(),
  user_id bigint not null references users,
  -- we want the most recent state of the world for the user's handling of a given question.
  unique (user_id, question_id, queue_id),
  -- update existing records to point to the latest answer. there should not be more than one
  -- record per (user, answer).
  unique (user_id, answer_id)
);

create trigger trigger_last_answers_set_updated_at_column before update on last_answers
  for each row execute procedure set_updated_at_column();

insert into users (handle, id)
  values ('gnusto', 1);

insert into questions (author_id, id, external_id, title, text, link, link_logo)
  values (
    1,
    1,
    'ya2',
    'How do I dump the data of some SQLite3 tables?',
    'Visit [this site](https://stackoverflow.com/questions/75675/how-do-i-dump-the-data-of-some-sqlite3-tables) and see how you do.',
    'https://stackoverflow.com/questions/75675/how-do-i-dump-the-data-of-some-sqlite3-tables',
    'https://cdn.sstatic.net/Sites/stackoverflow/Img/apple-touch-icon.png?v=c78bd457575a'
  );
