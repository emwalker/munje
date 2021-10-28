delete from answers;
alter table answers drop column answered_at;
alter table answers add column updated_at text not null;
