-- Add migration script here

create or replace function club_get_new_code() returns bigint language plpgsql as $$
    BEGIN
        RETURN code FROM (SELECT random+(SELECT COUNT(code) FROM club where code <= random) as code
        FROM random(0, (SELECT POW(10, GREATEST(ceil(log10(max_codes)), 4))::bigint-1-max_codes FROM (SELECT DISTINCT COUNT(code) FROM club) as v(max_codes)))) as c(code);
    END;
$$;

create or replace function club_create(self_discord_id bigint, code bigint, "club-path-name" text, public_key bytea, private_key bytea) returns void
    language sql
as
$$
WITH valid AS (
    SELECT
        club_create."club-path-name",
        club_create.code,
        club_create.public_key,
        club_create.private_key
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
) INSERT INTO club (code, "path-name", name, public_key, private_key) (SELECT valid.code, valid."club-path-name", valid."club-path-name", valid.public_key, valid.private_key FROM valid);
$$;

create or replace function discord_create(self_discord_id bigint, target_discord_id bigint, username text, discriminator smallint, display_name text) returns void
    language sql
as
$$
WITH valid AS (
    SELECT
        discord_create.target_discord_id,
        discord_create.username,
        discord_create.discriminator,
        discord_create.display_name
    FROM public.discord_info
    INNER JOIN public.discord_permissions ON public.discord_permissions.discord_id = discord_create.self_discord_id AND public.discord_permissions.club_id = 0
    WHERE
        public.discord_permissions.manage_permissions IS NOT NULL AND
        public.discord_permissions.manage_permissions = 0 AND
        (
            SELECT COUNT(*) FROM public.discord_info WHERE
                public.discord_info.user_id = discord_create.target_discord_id OR
                (public.discord_info.username = discord_create.username AND public.discord_info.discriminator = discord_create.discriminator)
        ) = 0
    LIMIT 1
) INSERT INTO discord_info (user_id, username, discriminator, display_name) (SELECT valid.target_discord_id, valid.username, valid.discriminator, valid.display_name FROM valid);
$$;