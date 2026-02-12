-- Add migration script here
create table discord_info
(
    user_id       bigint not null
        constraint discord_info_pk
            primary key,
    username      text   not null,
    discriminator smallint,
    display_name  text
);

create unique index discord_info_username_discriminator_uindex
    on discord_info (username, discriminator)
    nulls not distinct;


create table discord_avatar_info
(
    user_id bigint not null
        constraint discord_avatar_info_pk
            primary key
        constraint discord_avatar_info_discord_info_user_id_fk
            references discord_info
            on update restrict on delete cascade,
    animated bool not null,
    image_hash bytea not null
);

create table discord_permissions
(
    club_id                          bigint not null
        constraint discord_permissions_club_id_fk
            references club
            on update cascade on delete cascade,
    discord_id                       bigint not null
        constraint discord_permissions_discord_info_discord_id_fk
            references discord_info
            on update cascade on delete cascade,
    add_discord_user                 bool   not null,
    remove_discord_user              bool   not null,
    update_club_name                 bool   not null,
    update_allowed_code_replacements bool   not null,
    update_staff                     bool   not null,
    update_dancer                    bool   not null,
    update_lj                        bool   not null,
    update_dj                        bool   not null,
    update_hosts                     bool   not null,
    update_logo                      bool   not null,
    update_poster1                   bool   not null,
    update_poster2                   bool   not null,
    update_poster3                   bool   not null
);

create unique index discord_permissions_club_id_discord_id_uindex
    on discord_permissions (club_id, discord_id);

create index discord_permissions_club_id_index
    on discord_permissions (club_id);

create index discord_permissions_discord_id_index
    on discord_permissions (discord_id);

alter table discord_permissions
    add constraint discord_permissions_pk
        primary key (club_id, discord_id);
