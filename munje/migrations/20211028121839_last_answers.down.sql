drop table if exists last_answers;
alter table answers drop column stage;
alter table answers drop column consecutive_correct;
