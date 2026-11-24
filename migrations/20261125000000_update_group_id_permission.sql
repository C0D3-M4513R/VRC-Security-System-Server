-- Add migration script here
alter table discord_permissions
    add if not exists update_group_id boolean default false not null;

UPDATE discord_permissions
SET update_group_id = true
WHERE manage_permissions = 0;

CREATE OR REPLACE FUNCTION change_group_id(
    discord_id bigint,
    "club-path-name" text,
    group_id text
) returns
    void
    language sql
as
$$
WITH valid AS (
    SELECT id, CASE WHEN length(change_group_id.group_id) = 0 THEN null ELSE change_group_id.group_id END as group_id FROM club
                                                      INNER JOIN discord_permissions ON
        club.id = discord_permissions.club_id AND
        discord_permissions.discord_id = change_group_id.discord_id
    WHERE
        club."path-name" = "club-path-name" AND
        discord_permissions.update_group_id = true
)
UPDATE club
SET group_id = valid.group_id
FROM valid
WHERE club.id = valid.id
$$;

DROP FUNCTION IF EXISTS manage_permissions(
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
    manage_permissions1 integer,
    submit bool
);
CREATE OR REPLACE FUNCTION manage_permissions(
    self_discord_id bigint,
    target_discord_id bigint,
    "club-path-name" text,
    add_discord_user bool,
    remove_discord_user bool,
    update_club_name bool,
    update_group_id bool,
    add_allowed_code_replacements bool,
    add_level smallint,
    update_logo bool,
    update_poster1 bool,
    update_poster2 bool,
    update_poster3 bool,
    remove_allowed_code_replacements bool,
    remove_level smallint,
    manage_permissions1 integer,
    submit bool
) returns
    void
    language sql
as
$$

WITH valid AS (
    SELECT
        id,
        manage_permissions.target_discord_id as target_discord_id,
        CASE WHEN manage_permissions.add_discord_user IS NULL OR                        (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (self_perms.add_discord_user = false)                                         THEN COALESCE(target_perms.add_discord_user, false)                     ELSE manage_permissions.add_discord_user                    END as add_discord_user,
        CASE WHEN manage_permissions.remove_discord_user IS NULL OR                     (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (self_perms.remove_discord_user = false)                                      THEN COALESCE(target_perms.remove_discord_user, false)                  ELSE manage_permissions.remove_discord_user                 END as remove_discord_user,
        CASE WHEN manage_permissions.update_club_name IS NULL OR                        (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (self_perms.update_club_name = false)                                         THEN COALESCE(target_perms.update_club_name, false)                     ELSE manage_permissions.update_club_name                    END as update_club_name,
        CASE WHEN manage_permissions.update_group_id IS NULL OR                         (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (self_perms.update_group_id = false)                                            THEN COALESCE(target_perms.update_group_id, false)                      ELSE manage_permissions.update_group_id                     END as update_group_id,
        CASE WHEN manage_permissions.add_allowed_code_replacements IS NULL OR           (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (self_perms.add_allowed_code_replacements = false)                            THEN COALESCE(target_perms.add_allowed_code_replacements, false)        ELSE manage_permissions.add_allowed_code_replacements       END as add_allowed_code_replacements,
        CASE WHEN                                                                       (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (manage_permissions.add_level < self_perms.add_level)                         THEN target_perms.add_level                                             ELSE manage_permissions.add_level                           END as add_level,
        CASE WHEN manage_permissions.update_logo IS NULL OR                             (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (self_perms.update_logo = false)                                              THEN COALESCE(target_perms.update_logo, false)                          ELSE manage_permissions.update_logo                         END as update_logo,
        CASE WHEN manage_permissions.update_poster1 IS NULL OR                          (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (self_perms.update_poster1 = false)                                           THEN COALESCE(target_perms.update_poster1, false)                       ELSE manage_permissions.update_poster1                      END as update_poster1,
        CASE WHEN manage_permissions.update_poster2 IS NULL OR                          (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (self_perms.update_poster2 = false)                                           THEN COALESCE(target_perms.update_poster2, false)                       ELSE manage_permissions.update_poster2                      END as update_poster2,
        CASE WHEN manage_permissions.update_poster3 IS NULL OR                          (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (self_perms.update_poster3 = false)                                           THEN COALESCE(target_perms.update_poster3, false)                       ELSE manage_permissions.update_poster3                      END as update_poster3,
        CASE WHEN manage_permissions.remove_allowed_code_replacements IS NULL OR        (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (self_perms.remove_allowed_code_replacements = false)                         THEN COALESCE(target_perms.remove_allowed_code_replacements, false)     ELSE manage_permissions.remove_allowed_code_replacements    END as remove_allowed_code_replacements,
        CASE WHEN                                                                       (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (manage_permissions.remove_level < self_perms.remove_level)                   THEN target_perms.remove_level                                          ELSE manage_permissions.remove_level                        END as remove_level,
        CASE WHEN                                                                       (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (manage_permissions.manage_permissions1 <= self_perms.manage_permissions)     THEN target_perms.manage_permissions                                    ELSE manage_permissions.manage_permissions1                 END as manage_permissions,
        CASE WHEN manage_permissions.submit IS NULL OR                                  (target_perms.manage_permissions IS NOT NULL AND target_perms.manage_permissions <= self_perms.manage_permissions) OR (self_perms.submit = false)                                                   THEN COALESCE(target_perms.submit, false)                               ELSE manage_permissions.submit                              END as submit
    FROM club
         INNER JOIN discord_permissions AS self_perms ON
            (self_perms.club_id = club.id OR self_perms.club_id = 0) AND
            self_perms.discord_id = self_discord_id
         LEFT JOIN discord_permissions AS target_perms ON
            target_perms.club_id = club.id AND
            target_perms.discord_id = target_discord_id
    WHERE
        club."path-name" = "club-path-name" AND
        (target_perms.manage_permissions IS NULL OR self_perms.manage_permissions < target_perms.manage_permissions)
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
        update_group_id,
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
    ) VALUES (
                 valid.id,
                 valid.target_discord_id,
                 valid.add_discord_user,
                 valid.remove_discord_user,
                 valid.update_club_name,
                 valid.update_group_id,
                 valid.add_allowed_code_replacements,
                 valid.add_level,
                 valid.update_logo,
                 valid.update_poster1,
                 valid.update_poster2,
                 valid.update_poster3,
                 valid.remove_allowed_code_replacements,
                 valid.remove_level,
                 valid.manage_permissions,
                 valid.submit
             )
WHEN MATCHED THEN
    UPDATE SET
               add_discord_user = valid.add_discord_user,
               remove_discord_user = valid.remove_discord_user,
               update_club_name = valid.update_club_name,
               update_group_id = valid.update_group_id,
               add_allowed_code_replacements = valid.add_allowed_code_replacements,
               add_level = valid.add_level,
               update_logo = valid.update_logo,
               update_poster1 = valid.update_poster1,
               update_poster2 = valid.update_poster2,
               update_poster3 = valid.update_poster3,
               remove_allowed_code_replacements = valid.remove_allowed_code_replacements,
               remove_level = valid.remove_level,
               manage_permissions = valid.manage_permissions,
               submit = valid.submit
$$;