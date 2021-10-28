create table last_answers (
  user_id text not null,
  queue_id text not null,
  question_id text not null,
  answer_id text not null,
  answer_state text not null,
  answered_at text not null,
  consecutive_correct_answers int not null,
  answer_stage int not null,
  created_at text not null,
  updated_at text not null,
  foreign key (user_id) references users(id),
  foreign key (queue_id) references queues(id),
  foreign key (question_id) references questions(id),
  foreign key (answer_id) references answers(id),
  foreign key (answer_state) references answer_state(type),
  -- We want the most recent state of the world for the user's handling of a given question.
  -- We omit the queue for now, although it might make sense to add it later on.
  unique (user_id, question_id),
  -- Update existing records to point to the latest answer. There should not be more than one
  -- record per (user, answer).
  unique (user_id, answer_id)
);
