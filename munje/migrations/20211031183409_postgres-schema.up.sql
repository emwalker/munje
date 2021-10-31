create table users (
  id text not null primary key,
  handle text unique not null,
  created_at text not null,
  updated_at text not null
);

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

create table queues (
  id text not null primary key,
  starting_question_id text not null,
  user_id text not null,
  created_at text not null,
  updated_at text not null, title text, description text,
  foreign key (starting_question_id) references questions(id),
  foreign key (user_id) references users(id),
  unique (user_id, starting_question_id)
);

create table answer_states (
  type text not null primary key,
  "order" integer unique not null
);

insert into answer_states (type, "order")
  values ('correct', 0), ('incorrect', 1), ('unsure', 2);

create table answers (
  id text not null primary key,
  user_id text not null,
  queue_id text not null,
  question_id text not null,
  state text not null default('unstarted') references answer_states(type),
  created_at text not null, answered_at text, consecutive_correct int,
  foreign key (queue_id) references queues(id),
  foreign key (question_id) references questions(id),
  foreign key (user_id) references users(id)
);

create table last_answers (
  answer_answered_at text not null,
  answer_id text not null,
  answer_consecutive_correct int not null,
  answer_state text not null,
  created_at text not null,
  id text not null primary key,
  question_id text not null,
  queue_id text not null,
  updated_at text not null,
  user_id text not null,
  foreign key (user_id) references users(id),
  foreign key (queue_id) references queues(id),
  foreign key (question_id) references questions(id),
  foreign key (answer_id) references answers(id),
  foreign key (answer_state) references answer_states(type),
  -- we want the most recent state of the world for the user's handling of a given question.
  unique (user_id, question_id, queue_id),
  -- update existing records to point to the latest answer. there should not be more than one
  -- record per (user, answer).
  unique (user_id, answer_id)
);

insert into users (handle, id, created_at, updated_at)
  values (
    'gnusto', '21546b43-dcde-43b2-a251-e736194de0a0', '2021-10-30T18:28:06.396878840Z',
    '2021-10-30T18:28:06.396878840Z'
  );

insert into questions (id, author_id, title, text, link, link_logo, created_at, updated_at)
  values (
    'b4be00d2-9d37-4f8b-889b-705542bb03d2',
    '21546b43-dcde-43b2-a251-e736194de0a0',
    'How do I dump the data of some SQLite3 tables?',
    'Visit [this site](https://stackoverflow.com/questions/75675/how-do-i-dump-the-data-of-some-sqlite3-tables) and see how you do.',
    'https://stackoverflow.com/questions/75675/how-do-i-dump-the-data-of-some-sqlite3-tables',
    'https://cdn.sstatic.net/Sites/stackoverflow/Img/apple-touch-icon.png?v=c78bd457575a',
    '2021-10-30T18:28:06.396878840Z',
    '2021-10-30T18:28:06.396878840Z'
  );
