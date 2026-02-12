-- Add migration script here
drop function change_logo;
drop function change_poster1;
drop function change_poster2;
drop function change_poster3;
create or replace function change_logo(discord_id bigint, "club-path-name" text, image bytea)
    returns bytea
    language sql
as
$$
MERGE INTO club_logo
USING (
    SELECT DISTINCT id, change_logo.image as image FROM club
                                                            INNER JOIN discord_permissions ON
        (club.id = discord_permissions.club_id OR discord_permissions.club_id = 0) AND
        discord_permissions.discord_id = change_logo.discord_id
    WHERE
        club."path-name" = "club-path-name" AND
        discord_permissions.update_logo = true
) AS valid
ON valid.id = club_logo.club_id
WHEN MATCHED THEN
    UPDATE SET image = valid.image
WHEN NOT MATCHED THEN
    INSERT (club_id, image, digest) VALUES (valid.id, valid.image, digest(valid.image, 'sha3-512'))
    RETURNING digest
$$;


create or replace function change_poster1(discord_id bigint, "club-path-name" text, image bytea)
    returns bytea
    language sql
as
$$
MERGE INTO club_poster1
USING (
    SELECT id, change_poster1.image as image FROM club
                                                      INNER JOIN discord_permissions ON
        (club.id = discord_permissions.club_id OR discord_permissions.club_id = 0) AND
        discord_permissions.discord_id = change_poster1.discord_id
    WHERE
        club."path-name" = "club-path-name" AND
        discord_permissions.update_poster3 = true
) AS valid
ON valid.id = club_poster1.club_id
WHEN MATCHED THEN
    UPDATE SET image = valid.image
WHEN NOT MATCHED THEN
    INSERT (club_id, image, digest) VALUES (valid.id, valid.image, digest(valid.image, 'sha3-512'))
    RETURNING digest
$$;

create or replace function change_poster2(discord_id bigint, "club-path-name" text, image bytea)
    returns bytea
    language sql
as
$$
MERGE INTO club_poster2
USING (
    SELECT id, change_poster2.image as image FROM club
                                                      INNER JOIN discord_permissions ON
        (club.id = discord_permissions.club_id OR discord_permissions.club_id = 0) AND
        discord_permissions.discord_id = change_poster2.discord_id
    WHERE
        club."path-name" = "club-path-name" AND
        discord_permissions.update_poster3 = true
) AS valid
ON valid.id = club_poster2.club_id
WHEN MATCHED THEN
    UPDATE SET image = valid.image
WHEN NOT MATCHED THEN
    INSERT (club_id, image, digest) VALUES (valid.id, valid.image, digest(valid.image, 'sha3-512'))
RETURNING digest
$$;

create or replace function change_poster3(discord_id bigint, "club-path-name" text, image bytea)
    returns bytea
    language sql
as
$$
MERGE INTO club_poster3
USING (
    SELECT id, change_poster3.image as image FROM club
                                                      INNER JOIN discord_permissions ON
        (club.id = discord_permissions.club_id OR discord_permissions.club_id = 0) AND
        discord_permissions.discord_id = change_poster3.discord_id
    WHERE
        club."path-name" = "club-path-name" AND
        discord_permissions.update_poster3 = true
) AS valid
ON valid.id = club_poster3.club_id
WHEN MATCHED THEN
    UPDATE SET image = valid.image
WHEN NOT MATCHED THEN
    INSERT (club_id, image, digest) VALUES (valid.id, valid.image, digest(valid.image, 'sha3-512'))
    RETURNING digest
$$;