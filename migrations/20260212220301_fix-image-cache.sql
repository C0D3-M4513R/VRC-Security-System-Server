create or replace trigger club_logo_image_update_digest
    BEFORE UPDATE OF image, digest
    ON club_logo
    FOR EACH ROW
EXECUTE FUNCTION club_image_update_digest();
create or replace trigger club_poster1_image_update_digest
    BEFORE UPDATE OF image, digest
    ON club_poster1
    FOR EACH ROW
EXECUTE FUNCTION club_image_update_digest();
create or replace trigger club_poster2_image_update_digest
    BEFORE UPDATE OF image, digest
    ON club_poster2
    FOR EACH ROW
EXECUTE FUNCTION club_image_update_digest();
create or replace trigger club_poster3_image_update_digest
    BEFORE UPDATE OF image, digest
    ON club_poster3
    FOR EACH ROW
EXECUTE FUNCTION club_image_update_digest();