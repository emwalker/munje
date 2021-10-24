create table users (
  id text not null primary key,
  handle text unique not null,
  created_at text not null,
  updated_at text not null
);

insert into users (id, handle, created_at, updated_at)
  values ('21546b43-dcde-43b2-a251-e736194de0a0', 'gnusto', '2021-10-24T15:28:49.777100169+00:00',
    '2021-10-24T15:28:49.777100169+00:00');

create table queues (
  id text not null primary key,
  starting_question_id text not null,
  user_id text not null,
  created_at text not null,
  updated_at text not null,
  foreign key (starting_question_id) references questions(id),
  foreign key (user_id) references users(id),
  unique (user_id, starting_question_id)
);

create table answer_states (
  type text not null primary key,
  'order' integer unique not null
);

insert into answer_states (type, 'order') values ('unstarted', 0);
insert into answer_states (type, 'order') values ('started', 1);
insert into answer_states (type, 'order') values ('correct', 2);
insert into answer_states (type, 'order') values ('incorrect', 3);
insert into answer_states (type, 'order') values ('unsure', 4);
insert into answer_states (type, 'order') values ('skipped', 5);

create table answers (
  id text not null primary key,
  user_id text not null,
  queue_id text not null,
  question_id text not null,
  state text not null default('unstarted') references answer_states(type),
  created_at text not null,
  updated_at text not null,
  foreign key (queue_id) references queues(id),
  foreign key (question_id) references questions(id),
  foreign key (user_id) references users(id)
);

-- Add constraints to author_id and updated_at
drop table questions;

create table questions (
  id text not null primary key,
  author_id text not null,
  link text,
  link_logo text,
  created_at text not null,
  updated_at text not null,
  foreign key (author_id) references users(id)
);

insert into questions (id, author_id, link, link_logo, created_at, updated_at)
  values (
    '915430d6-960a-4c49-a87d-4f78ed3f059f',
    '21546b43-dcde-43b2-a251-e736194de0a0',
    'https://leetcode.com/problems/flower-planting-with-no-adjacent/',
    'https://assets.leetcode.com/static_assets/public/icons/favicon-96x96.png',
    '2021-10-24T15:28:49.777100169+00:00',
    '2021-10-24T15:28:49.777100169+00:00'
  );
