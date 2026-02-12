pub struct VRCPermission {
    pub vrc_name: String,
    pub permission_level: i16,
}
#[derive(askama::Template)]
#[template(path = "clubs/vrc-permissions.html")]
pub struct ClubVRCPermissions<'a> {
    pub information: super::club_instance::ClubInstance<'a>,
    pub permissions: Vec<VRCPermission>,
}