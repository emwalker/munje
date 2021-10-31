alter table answers add column stage int default(0) not null;
alter table last_answers add column answer_stage int default(0) not null;
