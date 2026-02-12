use std::borrow::Cow;

pub fn pull<'a>(
    remote: &'a mut git2::Remote,
) -> Result<(), Cow<'static, str>>{
    let refspecs = match remote.fetch_refspecs() {
        Ok(refspecs) => refspecs,
        Err(err) => {
            tracing::warn!("Failed to fetch refspecs for given remote: {err}");
            return Err(Cow::Owned(format!("Failed to fetch refspecs for given remote: {err}")));
        },
    };
    let mut refspec_vec = Vec::new();
    for refspec in refspecs.iter_bytes() {
        match core::str::from_utf8(refspec) {
            Ok(refspec) => refspec_vec.push(refspec),
            Err(err) => {
                tracing::info!("Skipping refspec, which failed to parse as utf-8 '{refspec:?}': {err}");
                continue;
            }
        }
    }
    let mut remote_callbacks = git2::RemoteCallbacks::new();
    super::auth::add_auth(&mut remote_callbacks)?;
    let mut fetch_opts = git2::FetchOptions::default();
    fetch_opts.remote_callbacks(remote_callbacks);
    fetch_opts.prune(git2::FetchPrune::On);
    fetch_opts.update_fetchhead(true);
    match remote.fetch(refspec_vec.as_slice(), Some(&mut fetch_opts), None) {
        Ok(()) => (),
        Err(err) => {
            tracing::warn!("Failed to fetch for given remote: {err}");
            return Err(Cow::Owned(format!("Failed to fetch for given remote: {err}")));
        }
    }

    Ok(())
}
