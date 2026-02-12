use crate::rocket::api::club::Permissions;


#[derive(askama::Template)]
#[template(path = "clubs/instance.html")]
pub struct ClubInstance {
    pub name: String,
    pub path_name: String,
    pub permissions: Permissions
}