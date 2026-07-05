use std::sync::Arc;
use crate::rocket::api::club::Permissions;

#[derive(Debug, Clone)]
pub struct Club {
    pub name: Arc<str>,
    pub code: u64,
    pub path_name: Arc<str>,
}

#[derive(askama::Template)]
#[template(path = "clubs/index.html")]
pub struct Clubs {
    pub clubs_id: Vec<Club>,
    pub clubs_name: Vec<Club>,
    pub permission: Option<Permissions>
}
impl Clubs {
    pub fn new(mut clubs: Vec<Club>, permission: Option<Permissions>) -> Self {
        Self {
            clubs_id: clubs.clone(),
            clubs_name: {
                clubs.sort_by(|a, b|core::cmp::Ord::cmp(&a.name, &b.name));
                clubs
            },
            permission,
        }
    }
}