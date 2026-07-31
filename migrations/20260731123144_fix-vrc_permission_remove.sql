-- Add migration script here
create or replace function remove_vrcuser_level(self_discord_id bigint, "club-path-name" text, vrc_username text, level integer) returns void
    language sql
as
$$
WITH valid as (
        SELECT
            club.id as club_id,
            remove_vrcuser_level.level as level
        FROM club
                 INNER JOIN discord_permissions ON
            (club.id = discord_permissions.club_id OR discord_permissions.club_id = 0) AND
            discord_permissions.discord_id = self_discord_id
        WHERE
            club."path-name" = "club-path-name" AND
            discord_permissions.remove_level <= remove_vrcuser_level.level
    ), vrc_id AS (
        SELECT * from public.vrc_name WHERE public.vrc_name.name = remove_vrcuser_level.vrc_username
    )
    DELETE FROM public.club_vrc_permission
           USING valid, vrc_id
           WHERE
               public.club_vrc_permission.club_id = valid.club_id AND
               public.club_vrc_permission.vrc_name = vrc_id.id AND
               vrc_id.name = remove_vrcuser_level.vrc_username AND
               public.club_vrc_permission.permission_level = valid.level;
$$;