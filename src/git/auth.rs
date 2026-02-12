use std::borrow::Cow;
use std::ffi::OsString;
use std::path::Path;

pub fn add_auth(callbacks: &mut git2::RemoteCallbacks) -> Result<(), Cow<'static, str>> {
    static GIT_REPO_PUBLICKEY_PATH:std::sync::LazyLock<Result<OsString, &str>> = std::sync::LazyLock::new(||std::env::var_os("GIT_REPO_PUBLICKEY_PATH").ok_or("GIT_REPO_PUBLICKEY_PATH environment variable not set"));
    static GIT_REPO_PRIVATEKEY_PATH:std::sync::LazyLock<Result<OsString, &str>> = std::sync::LazyLock::new(||std::env::var_os("GIT_REPO_PRIVATEKEY_PATH").ok_or("GIT_REPO_PRIVATEKEY_PATH environment variable not set"));

    let publickey = GIT_REPO_PUBLICKEY_PATH.as_ref().map_err(|v|Cow::Borrowed(*v))?;
    let privatekey = GIT_REPO_PRIVATEKEY_PATH.as_ref().map_err(|v|Cow::Borrowed(*v))?;

    callbacks.credentials(move |url, username, allowed_types|{
        tracing::info!("Git: Auth request for {url} {username:?} {allowed_types:?}");
        if !allowed_types.is_ssh_key() {
            return Err(git2::Error::new(git2::ErrorCode::NotFound,git2::ErrorClass::Callback, "Only SSH keys are supported."));
        }
        git2::Cred::ssh_key(
            username.unwrap_or("git"), //Todo: is this correct?
            Some(Path::new(&publickey)),
            Path::new(&privatekey),
            None
        )
    });
    Ok(())
}