create table club
(
    id          bigint generated always as identity,
    code        bigint not null,
    "path-name" text   not null,
    name        text   not null,
    public_key  bytea not null,
    private_key bytea not null
);

create unique index club_code_uindex
    on club (code);

create unique index club_id_uindex
    on club (id);

create unique index "club_path-name_uindex"
    on club ("path-name");

alter table club
    add constraint club_pk
        primary key (id);