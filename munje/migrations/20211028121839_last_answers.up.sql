create table last_answers (
  answer_answered_at text not null,
  answer_id text not null,
  answer_consecutive_correct int not null,
  answer_stage int not null,
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
  -- We want the most recent state of the world for the user's handling of a given question.
  unique (user_id, question_id, queue_id),
  -- Update existing records to point to the latest answer. There should not be more than one
  -- record per (user, answer).
  unique (user_id, answer_id)
);

alter table answers add column consecutive_correct int;
alter table answers add column stage int;
