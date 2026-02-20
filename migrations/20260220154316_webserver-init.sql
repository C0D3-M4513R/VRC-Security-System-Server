-- Add migration script here
CREATE OR REPLACE FUNCTION add_initial_club(self_discord_id bigint, public_key bytea, private_key bytea) returns void language sql
as $$
WITH valid as (
    SELECT
        0 as id,
        0 as code,
        '!CLUB-OWNERS' as "path-name",
        '!CLUB-OWNERS' as name,
        add_initial_club.public_key,
        add_initial_club.private_key,
        add_initial_club.self_discord_id
    WHERE NOT EXISTS(SELECT id FROM club)
), insert as (
    INSERT INTO club OVERRIDING SYSTEM VALUE (SELECT valid.id, valid.code, valid."path-name", valid.name, valid.public_key, valid.private_key from valid)
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
$$