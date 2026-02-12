pub struct Club {
    pub name: String,
    pub path_name: String,
}

#[derive(askama::Template)]
#[template(path = "clubs/index.html")]
pub struct Clubs {
    pub clubs: Vec<Club>,
}