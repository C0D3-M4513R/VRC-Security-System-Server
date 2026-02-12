-- Add migration script here
drop extension if exists pgcrypto;
create extension pgcrypto;

alter table club_logo
    add digest bytea;

alter table club_poster1
    add digest bytea;

alter table club_poster2
    add digest bytea;

alter table club_poster3
    add digest bytea;

UPDATE public.club_logo SET digest = digest(public.club_logo.image, 'sha3-512');
UPDATE public.club_poster1 SET digest = digest(public.club_poster1.image, 'sha3-512');
UPDATE public.club_poster2 SET digest = digest(public.club_poster2.image, 'sha3-512');
UPDATE public.club_poster3 SET digest = digest(public.club_poster3.image, 'sha3-512');

alter table club_logo
    alter column digest
        set not null;

alter table club_poster1
    alter column digest
        set not null;

alter table club_poster2
    alter column digest
        set not null;

alter table club_poster3
    alter column digest
        set not null;

create or replace function club_image_update_digest()
    returns trigger
    language plpgsql
as $$
BEGIN
    NEW.digest := digest(NEW.image, 'sha3-512');
    RETURN NEW;
END;
$$;

create or replace trigger club_logo_image_update_digest
    AFTER UPDATE OF image, digest
    ON club_logo
    FOR EACH ROW
EXECUTE FUNCTION club_image_update_digest();

create or replace trigger club_poster1_image_update_digest
    AFTER UPDATE OF image, digest
    ON club_poster1
    FOR EACH ROW
EXECUTE FUNCTION club_image_update_digest();
create or replace trigger club_poster2_image_update_digest
    AFTER UPDATE OF image, digest
    ON club_poster2
    FOR EACH ROW
EXECUTE FUNCTION club_image_update_digest();
create or replace trigger club_poster3_image_update_digest
    AFTER UPDATE OF image, digest
    ON club_poster3
    FOR EACH ROW
EXECUTE FUNCTION club_image_update_digest();