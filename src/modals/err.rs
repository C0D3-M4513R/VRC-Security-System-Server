use std::borrow::Cow;

#[derive(askama::Template)]
#[template(path = "err.html")]
pub struct Err<'r> {
    pub error: Cow<'r, str>,
    pub error_description: Option<Cow<'r, str>>,
}
impl<'r> Err<'r> {
    #[inline]
    pub fn get_error_description(&'r self) -> &'r str {
        match &self.error_description {
            Some(desc) => &**desc,
            None => "",
        }
    }
}