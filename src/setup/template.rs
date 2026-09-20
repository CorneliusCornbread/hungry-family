use askama::Template;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "setup.html")]
pub(crate) struct SetupPage<'a> {
    pub(crate) form: &'a SetupForm,
    pub(crate) error: Option<&'a str>,
}

#[derive(Template)]
#[template(path = "setup_form.html")]
pub(crate) struct SetupFormFragment<'a> {
    form: &'a SetupForm,
    error: Option<&'a str>,
}

// Needed so fields are accessible in the template's `{% include %}`
#[derive(Deserialize, Default)]
pub struct SetupForm {
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub password: String,
}
