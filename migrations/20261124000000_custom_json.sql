-- Add migration script here
alter table club add if not exists custom_json jsonb;
