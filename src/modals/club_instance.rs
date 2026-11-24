use crate::Limits;
use crate::rocket::api::club::Permissions;

pub struct ClubInfo {
    pub name: String,
    pub path_name: String,
    pub permissions: Permissions,
    pub limits: Limits,
}

#[derive(askama::Template)]
#[template(path = "clubs/instance.html")]
pub struct ClubInstance {
    pub info: ClubInfo,
    pub code: i64,
    pub group_id: Option<String>,
    pub custom_json: Option<String>,
}