create table club_logo
(
    club_id bigint not null
        constraint club_logo_pk
            primary key
        constraint club_logo_club_id_fk
            references club
            on update restrict on delete cascade,
    image   bytea  not null
);
create table club_poster1
(
    club_id bigint not null
        constraint club_poster1_pk
            primary key
        constraint club_poster1_club_id_fk
            references club
            on update restrict on delete cascade,
    image   bytea  not null
);
create table club_poster2
(
    club_id bigint not null
        constraint club_poster2_pk
            primary key
        constraint club_poster2_club_id_fk
            references club
            on update restrict on delete cascade,
    image   bytea  not null
);
create table club_poster3
(
    club_id bigint not null
        constraint club_poster3_pk
            primary key
        constraint club_poster3_club_id_fk
            references club
            on update restrict on delete cascade,
    image   bytea  not null
);

create function change_poster3(
    discord_id bigint,
    "club-path-name" text,
    image bytea
) returns
    void
    language sql
as
$$
MERGE INTO club_poster3
USING (
    SELECT id, change_poster3.image as image FROM club
                                                      INNER JOIN discord_permissions ON
        club.id = discord_permissions.club_id AND
        discord_permissions.discord_id = change_poster3.discord_id
    WHERE
        club."path-name" = "club-path-name" AND
        discord_permissions.update_poster3 = true
) AS valid
ON valid.id = club_poster3.club_id
WHEN MATCHED THEN
    UPDATE SET image = valid.image
WHEN NOT MATCHED THEN
    INSERT (club_id, image) VALUES (valid.id, valid.image)
$$;
create table vrc_name
(
    id   bigint generated always as identity
        constraint vrc_name_pk
            primary key,
    name text not null
);

create unique index vrc_name_id_uindex
    on vrc_name (id);

create unique index vrc_name_name_uindex
    on vrc_name (name);

create table club_vrc_permission
(
    club_id          bigint   not null
        constraint club_vrc_permission_club_id_fk
            references club,
    vrc_name         bigint   not null
        constraint club_vrc_permission_vrc_name_id_fk
            references vrc_name,
    permission_level smallint not null,
    constraint club_vrc_permission_pk
        primary key (club_id, vrc_name, permission_level)
);

create index club_vrc_permission_club_id_index
    on club_vrc_permission (club_id);

create index club_vrc_permission_club_id_permission_level_index
    on club_vrc_permission (club_id, permission_level);


create table club_allowed_replace
(
    club_id          bigint not null
        constraint club_allowed_replace_club_id_fk
            references club
            on update restrict on delete cascade,
    switch_to_clubid bigint not null
        constraint club_allowed_replace_club_id_fk_2
            references club
            on update restrict on delete cascade,
    constraint club_allowed_replace_pk
        primary key (club_id, switch_to_clubid)
);

create function change_club_name(
    discord_id bigint,
    "club-path-name" text,
    name text
) returns
    void
    language sql
as
$$
WITH valid AS (
    SELECT id, change_club_name.name as name FROM club
                                                      INNER JOIN discord_permissions ON
        club.id = discord_permissions.club_id AND
        discord_permissions.discord_id = change_club_name.discord_id
    WHERE
        club."path-name" = "club-path-name" AND
        discord_permissions.update_club_name = true
)
UPDATE club
SET name = valid.name
FROM valid
WHERE club.id = valid.id
$$;

begin transaction;
DROP TABLE discord_permissions;
create table discord_permissions
(
    club_id                          bigint                not null
        constraint discord_permissions_club_id_fk
            references club
            on update cascade on delete cascade,
    discord_id                       bigint                not null
        constraint discord_permissions_discord_info_discord_id_fk
            references discord_info
            on update cascade on delete cascade,
    add_discord_user                 boolean default false not null,
    remove_discord_user              boolean default false not null,
    update_club_name                 boolean default false not null,
    add_allowed_code_replacements    boolean default false not null,
    add_level                        smallint default null,
    update_logo                      boolean default false not null,
    update_poster1                   boolean default false not null,
    update_poster2                   boolean default false not null,
    update_poster3                   boolean default false not null,
    remove_allowed_code_replacements boolean default false not null,
    remove_level                     smallint default null,
    manage_permissions               integer not null,
    constraint discord_permissions_pk
        primary key (club_id, discord_id)
);

alter table discord_permissions
    owner to neoluma;

create unique index discord_permissions_club_id_discord_id_uindex
    on discord_permissions (club_id, discord_id);

create index discord_permissions_club_id_index
    on discord_permissions (club_id);

create index discord_permissions_discord_id_index
    on discord_permissions (discord_id);

commit;

create or replace function manage_permissions(
    self_discord_id bigint,
    target_discord_id bigint,
    "club-path-name" text,
    add_discord_user bool,
    remove_discord_user bool,
    update_club_name bool,
    add_allowed_code_replacements bool,
    add_level smallint,
    update_logo bool,
    update_poster1 bool,
    update_poster2 bool,
    update_poster3 bool,
    remove_allowed_code_replacements bool,
    remove_level smallint,
    manage_permissions1 integer
) returns
    void
    language sql
as
$$
WITH valid AS (
    SELECT
        id,
        manage_permissions.target_discord_id as target_discord_id,
        CASE WHEN manage_permissions.add_discord_user IS NULL OR                        (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (target_perms.add_discord_user IS NOT NULL AND self_perms.add_discord_user = false)                                        THEN target_perms.add_discord_user                   ELSE manage_permissions.add_discord_user                  END as add_discord_user,
        CASE WHEN manage_permissions.remove_discord_user IS NULL OR                     (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (target_perms.remove_discord_user IS NOT NULL AND self_perms.remove_discord_user = false)                                  THEN target_perms.remove_discord_user                ELSE manage_permissions.remove_discord_user               END as remove_discord_user,
        CASE WHEN manage_permissions.update_club_name IS NULL OR                        (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (target_perms.update_club_name IS NOT NULL AND self_perms.update_club_name = false)                                        THEN target_perms.update_club_name                   ELSE manage_permissions.update_club_name                  END as update_club_name,
        CASE WHEN manage_permissions.add_allowed_code_replacements IS NULL OR           (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (target_perms.add_allowed_code_replacements IS NOT NULL AND self_perms.add_allowed_code_replacements = false)              THEN target_perms.add_allowed_code_replacements      ELSE manage_permissions.add_allowed_code_replacements     END as add_allowed_code_replacements,
        CASE WHEN manage_permissions.add_level IS NULL OR                               (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (target_perms.add_level IS NOT NULL AND target_perms.add_level <= self_perms.add_level)                                     THEN target_perms.add_level                          ELSE manage_permissions.add_level                         END as add_level,
        CASE WHEN manage_permissions.update_logo IS NULL OR                             (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (target_perms.update_logo IS NOT NULL AND NOT self_perms.update_logo = false)                                              THEN target_perms.update_logo                        ELSE manage_permissions.update_logo                       END as update_logo,
        CASE WHEN manage_permissions.update_poster1 IS NULL OR                          (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (target_perms.update_poster1 IS NOT NULL AND NOT self_perms.update_poster1 = false)                                        THEN target_perms.update_poster1                     ELSE manage_permissions.update_poster1                    END as update_poster1,
        CASE WHEN manage_permissions.update_poster2 IS NULL OR                          (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (target_perms.update_poster2 IS NOT NULL AND NOT self_perms.update_poster2 = false)                                        THEN target_perms.update_poster2                     ELSE manage_permissions.update_poster2                    END as update_poster2,
        CASE WHEN manage_permissions.update_poster3 IS NULL OR                          (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (target_perms.update_poster3 IS NOT NULL AND NOT self_perms.update_poster3 = false)                                        THEN target_perms.update_poster3                     ELSE manage_permissions.update_poster3                    END as update_poster3,
        CASE WHEN manage_permissions.remove_allowed_code_replacements IS NULL OR        (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (target_perms.remove_allowed_code_replacements IS NOT NULL AND NOT self_perms.remove_allowed_code_replacements = false)    THEN target_perms.remove_allowed_code_replacements   ELSE manage_permissions.remove_allowed_code_replacements  END as remove_allowed_code_replacements,
        CASE WHEN manage_permissions.remove_level IS NULL OR                            (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (target_perms.remove_level IS NOT NULL AND target_perms.remove_level <= self_perms.remove_level)                            THEN target_perms.remove_level                       ELSE manage_permissions.remove_level                      END as remove_level,
        CASE WHEN manage_permissions.manage_permissions1 IS NULL OR                     (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions)          THEN target_perms.manage_permissions                 ELSE manage_permissions.manage_permissions1               END as manage_permissions
    FROM club
             INNER JOIN discord_permissions AS self_perms ON
        self_perms.club_id = club.id AND
        self_perms.discord_id = self_discord_id
             LEFT JOIN discord_permissions AS target_perms ON
        target_perms.club_id = club.id AND
        target_perms.discord_id = target_discord_id
    WHERE
        club."path-name" = "club-path-name" AND
        (target_perms.manage_permissions IS NULL OR target_perms.manage_permissions <= self_perms.manage_permissions)
    )
    MERGE INTO discord_permissions
USING valid
ON
    discord_permissions.club_id = valid.id AND
    discord_permissions.discord_id = target_discord_id
WHEN NOT MATCHED THEN
    INSERT (
        club_id,
        discord_id,
        add_discord_user,
        remove_discord_user,
        update_club_name,
        add_allowed_code_replacements,
        add_level,
        update_logo,
        update_poster1,
        update_poster2,
        update_poster3,
        remove_allowed_code_replacements,
        remove_level,
        manage_permissions
    ) VALUES (
                 valid.id,
                 valid.target_discord_id,
                 valid.add_discord_user,
                 valid.remove_discord_user,
                 valid.update_club_name,
                 valid.add_allowed_code_replacements,
                 valid.add_level,
                 valid.update_logo,
                 valid.update_poster1,
                 valid.update_poster2,
                 valid.update_poster3,
                 valid.remove_allowed_code_replacements,
                 valid.remove_level,
                 valid.manage_permissions
             )
WHEN MATCHED THEN
    UPDATE SET
               add_discord_user = valid.add_discord_user,
               remove_discord_user = valid.remove_discord_user,
               update_club_name = valid.update_club_name,
               add_allowed_code_replacements = valid.add_allowed_code_replacements,
               add_level = valid.add_level,
               update_logo = valid.update_logo,
               update_poster1 = valid.update_poster1,
               update_poster2 = valid.update_poster2,
               update_poster3 = valid.update_poster3,
               remove_allowed_code_replacements = valid.remove_allowed_code_replacements,
               remove_level = valid.remove_level,
               manage_permissions = valid.manage_permissions
$$;

create or replace function remove_vrcuser_level(
    self_discord_id bigint,
    "club-path-name" text,
    vrc_username text,
    level integer
) returns
    void
    language sql
as
$$
WITH valid as (
    SELECT
        club.id as club_id,
        vrc_name.id as vrc_name_id,
        remove_vrcuser_level.level as level
    FROM club, vrc_name
                   INNER JOIN discord_permissions ON
        id = discord_permissions.club_id AND
        discord_permissions.discord_id = self_discord_id
    WHERE
        club."path-name" = "club-path-name" AND
        vrc_name.name = vrc_username AND
        discord_permissions.remove_level >= remove_vrcuser_level.level
) DELETE
FROM club_vrc_permission
    USING valid
WHERE
    club_vrc_permission.club_id = valid.club_id AND
    club_vrc_permission.vrc_name = valid.vrc_name_id AND
    club_vrc_permission.permission_level = valid.level
$$;

create or replace function add_vrcuser_level(
    self_discord_id bigint,
    "club-path-name" text,
    vrc_username text,
    level integer
) returns
    void
    language sql
as
$$
WITH valid as (
    WITH valid as (
        SELECT
            club.id as club_id,
            add_vrcuser_level.vrc_username as vrc_username,
            add_vrcuser_level.level as level
        FROM club
                 INNER JOIN discord_permissions ON
            id = discord_permissions.club_id AND
            discord_permissions.discord_id = self_discord_id
        WHERE
            club."path-name" = "club-path-name" AND
            discord_permissions.remove_level >= add_vrcuser_level.level
        ) MERGE INTO vrc_name
        USING valid
        ON valid.vrc_username = vrc_name.name
        WHEN MATCHED THEN DO NOTHING
        WHEN NOT MATCHED THEN
            INSERT (name) VALUES (valid.vrc_username)
            RETURNING
                vrc_name.id as vrc_id,
                valid.club_id as club_id,
                valid.level as level
) INSERT INTO club_vrc_permission
SELECT club_id, vrc_id as vrc_name, level as permission_level from valid
$$;

create unique index club_name_uindex
    on club (name);

alter table club
    add constraint club_code_unique_key
        unique (code);

alter table club
    add constraint club_name_unique_key
        unique (name);

alter table club
    add constraint "club_path-name_unique_key"
        unique ("path-name");

create or replace function add_allowed_code_replacements(
    self_discord_id bigint,
    "self-club-path-name" text,
    "new-club-path-name" text
) returns
    void
    language sql
as
$$
WITH valid as (
    WITH valid as (
        SELECT
            id as club_id
        FROM club
                 INNER JOIN discord_permissions ON
            club.id = discord_permissions.club_id AND
            discord_permissions.discord_id = self_discord_id
        WHERE club."path-name" = "self-club-path-name"
          AND discord_permissions.add_allowed_code_replacements = true
    ) SELECT
          valid.club_id,
          club.id as new_club_id
    FROM valid, club
    WHERE club."path-name" = "new-club-path-name"
    )
    MERGE INTO club_allowed_replace
USING valid
ON
    valid.club_id = club_allowed_replace.club_id AND
    valid.new_club_id = club_allowed_replace.switch_to_clubid
WHEN MATCHED THEN DO NOTHING
WHEN NOT MATCHED THEN
    INSERT (club_id, switch_to_clubid) VALUES (valid.club_id, valid.new_club_id)
$$;

create or replace function remove_allowed_code_replacements(
    self_discord_id bigint,
    "self-club-path-name" text,
    "new-club-path-name" text
) returns
    void
    language sql
as
$$
WITH valid as (
    WITH valid as (
        SELECT
            id as club_id
        FROM club
                 INNER JOIN discord_permissions ON
            club.id = discord_permissions.club_id AND
            discord_permissions.discord_id = self_discord_id
        WHERE club."path-name" = "self-club-path-name"
          AND discord_permissions.remove_allowed_code_replacements = true
    ) SELECT
          valid.club_id,
          club.id as new_club_id
    FROM valid, club
    WHERE club."path-name" = "new-club-path-name"
)
DELETE FROM club_allowed_replace
    USING valid
WHERE
    valid.club_id = club_allowed_replace.club_id AND
    valid.new_club_id = club_allowed_replace.switch_to_clubid
$$;

alter table discord_permissions
    alter column manage_permissions drop not null;

alter table discord_permissions
    add submit bool default false not null;

