use crate::rocket::api::club::Permissions;

pub struct DiscordPermission {
    pub discord_id: u64,
    pub discord_name: String,
    pub discord_discriminator: Option<i16>,
    // pub discord_display_name: Option<String>,
    pub permission: Permissions,
}
#[derive(askama::Template)]
#[template(path = "clubs/manage-discord-permissions.html")]
pub struct ClubDiscordPermissions {
    pub information: super::club_instance::ClubInstance,
    pub permissions: Vec<DiscordPermission>,
}