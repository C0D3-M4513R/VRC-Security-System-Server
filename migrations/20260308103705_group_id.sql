-- Add migration script here
alter table club
    add if not exists group_id text;
DROP FUNCTION IF EXISTS add_initial_club(self_discord_id bigint, public_key bytea, private_key bytea);
CREATE OR REPLACE FUNCTION add_initial_club(self_discord_id bigint, public_key bytea, private_key bytea, group_id text) returns void language sql
as $$
WITH valid as (
    SELECT
        0 as id,
        0 as code,
        '!CLUB-OWNERS' as "path-name",
        '!CLUB-OWNERS' as name,
        add_initial_club.public_key,
        add_initial_club.private_key,
        add_initial_club.self_discord_id,
        add_initial_club.group_id
    WHERE NOT EXISTS(SELECT id FROM club)
), insert as (
    INSERT INTO club OVERRIDING SYSTEM VALUE (SELECT valid.id, valid.code, valid."path-name", valid.name, valid.public_key, valid.private_key, valid.group_id from valid)
        returning club.*
) INSERT INTO discord_permissions (
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
    manage_permissions,
    submit
) (
    SELECT
        insert.id,
        valid.self_discord_id,
        true,
        true,
        true,
        true,
        0,
        true,
        true,
        true,
        true,
        true,
        0,
        0,
        true
    FROM insert
             INNER JOIN valid ON insert.id = valid.id
)
$$;

drop function IF EXISTS club_create(self_discord_id bigint, code bigint, "club-path-name" text, public_key bytea, private_key bytea);
create or replace function club_create(self_discord_id bigint, code bigint, "club-path-name" text, public_key bytea, private_key bytea, group_id text) returns void
    language sql
as
$$
WITH valid AS (
    SELECT
        club_create."club-path-name",
        club_create.code,
        club_create.public_key,
        club_create.private_key,
        CASE WHEN length(club_create.group_id) = 0 then null ELSE club_create.group_id END
    FROM public.discord_permissions
    WHERE
        public.discord_permissions.club_id = 0 AND
        public.discord_permissions.discord_id = club_create.self_discord_id AND
        public.discord_permissions.manage_permissions IS NOT NULL AND
        public.discord_permissions.manage_permissions = 0 AND
        NOT starts_with(club_create."club-path-name", '!') AND
        to_ascii(club_create."club-path-name", 'LATIN1') = club_create."club-path-name" AND
        (
            SELECT COUNT(*) FROM public.club WHERE
                public.club."path-name" = club_create."club-path-name" OR
                public.club.code = club_create.code
        ) = 0
) INSERT INTO club (code, "path-name", name, public_key, private_key, group_id) (SELECT valid.code, valid."club-path-name", valid."club-path-name", valid.public_key, valid.private_key, valid.group_id FROM valid);
$$;