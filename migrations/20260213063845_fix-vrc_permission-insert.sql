-- Add migration script here
create or replace function add_vrcuser_level(self_discord_id bigint, "club-path-name" text, vrc_username text, level integer) returns void
    language sql
as
$$
WITH valid as (
        SELECT
            club.id as club_id,
            add_vrcuser_level.vrc_username as vrc_username,
            add_vrcuser_level.level as level
        FROM club
                 INNER JOIN discord_permissions ON
            (club.id = discord_permissions.club_id OR discord_permissions.club_id = 0) AND
            discord_permissions.discord_id = self_discord_id
        WHERE
            club."path-name" = "club-path-name" AND
            discord_permissions.add_level <= add_vrcuser_level.level
    ), vrc_id_existing AS (
        SELECT * from public.vrc_name
        INNER JOIN valid ON valid.vrc_username = public.vrc_name.name
    ), vrc_id_inserted AS (
        INSERT INTO public.vrc_name (name) (
            SELECT valid.vrc_username FROM valid
            WHERE NOT EXISTS(SELECT 1 from vrc_id_existing WHERE vrc_id_existing.name = valid.vrc_username)
        ) RETURNING public.vrc_name.id as vrc_id, public.vrc_name.name as vrc_username
    ), vrc_id as (
        SELECT vrc_id_inserted.vrc_id, vrc_id_inserted.vrc_username FROM vrc_id_inserted
        UNION ALL
        SELECT vrc_id_existing.id as vrc_id, vrc_id_existing.name as vrc_username FROM vrc_id_existing
    ), valid_id as (
        SELECT valid.club_id, vrc_id.vrc_id, valid.level FROM valid
        INNER JOIN vrc_id ON valid.vrc_username = vrc_id.vrc_username
    )
    MERGE INTO public.club_vrc_permission
USING valid_id ON
    public.club_vrc_permission.club_id = valid_id.club_id AND
    public.club_vrc_permission.vrc_name = valid_id.vrc_id AND
    public.club_vrc_permission.permission_level = valid_id.level
WHEN MATCHED THEN DO NOTHING
WHEN NOT MATCHED THEN INSERT (club_id, vrc_name, permission_level) VALUES (valid_id.club_id, valid_id.vrc_id, valid_id.level)
$$;