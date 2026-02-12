use std::borrow::Cow;
use crate::git::{auth, pull, search_branch};
pub fn add_files_top(
    files: Vec<(&[u8], &str)>,
    repo: &git2::Repository,
    builder: &mut git2::TreeBuilder<'_>
) -> Result<git2::Oid, Cow<'static, str>> {
    for (bytes, name) in files {
        let oid = match repo.blob(bytes) {
            Ok(oid) => oid,
            Err(err) => {
                tracing::warn!("Could not convert file content to oid for file {name}: {err}");
                return Err(Cow::Owned(format!("Could not convert file content to oid for file {name}: {err}")));
            }
        };
        let _ = match builder.insert(name, oid, 0o100644) {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!("Could not insert blob-oid to club tree-builder for file {name}: {err}");
                return Err(Cow::Owned(format!("Could not insert blob-oid to club tree-builder for file {name}: {err}")));
            }
        };
    }

    let oid = match builder.write() {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Could not write Club-Tree to repo: {err}");
            return Err(Cow::Owned(format!("Could not write Club-Tree to repo: {err}")));
        }
    };

    Ok(oid)
}
pub fn add_files(
    files: Vec<(&[u8], &str)>,
    club_name: &str,
) -> impl FnOnce(&git2::Repository, &mut git2::TreeBuilder<'_>) -> Result<(), Cow<'static, str>> {
    move |repo, builder|{
        const CODES_NAME:&str = "Codes";
        let codes_oid = {
            let codes = match builder.get(CODES_NAME) {
                Ok(Some(v)) => v,
                Ok(None) => {
                    tracing::warn!("The Codes Tree was empty");
                    return Err(Cow::Borrowed("The Codes Tree was empty"));
                }
                Err(err) => {
                    tracing::warn!("Could not get Codes TreeEntry: {err}");
                    return Err(Cow::Owned(format!("Could not get Codes TreeEntry: {err}")));
                },
            };
            let codes = match codes.to_object(&repo) {
                Ok(c) => c,
                Err(err) => {
                    tracing::warn!("Could not get Object of Codes Tree: {err}");
                    return Err(Cow::Owned(format!("Could not get Object of Codes Tree: {err}")));
                },
            };
            let codes = match codes.peel_to_tree() {
                Ok(c) => c,
                Err(err) => {
                    tracing::warn!("Could not get Codes Tree: {err}");
                    return Err(Cow::Owned(format!("Could not get Codes Tree: {err}")));
                },
            };
            let mut codes = match repo.treebuilder(Some(&codes))  {
                Ok(c) => c,
                Err(err) => {
                    tracing::warn!("Could not get Codes TreeBuilder: {err}");
                    return Err(Cow::Owned(format!("Could not get Codes TreeBuilder: {err}")));
                },
            };
            let club_oid = {
                let club = match codes.get(club_name) {
                    Ok(Some(v)) => v,
                    Ok(None) => {
                        tracing::warn!("The Club Tree was empty");
                        return Err(Cow::Borrowed("The Club Tree was empty"));
                    }
                    Err(err) => {
                        tracing::warn!("Could not get Club Tree: {err}");
                        return Err(Cow::Owned(format!("Could not get Club Tree: {err}")));
                    },
                };
                let club = match club.to_object(&repo) {
                    Ok(c) => c,
                    Err(err) => {
                        tracing::warn!("Could not get Object of Club Tree: {err}");
                        return Err(Cow::Owned(format!("Could not get Object of Club Tree: {err}")));
                    },
                };
                let club = match club.peel_to_tree() {
                    Ok(c) => c,
                    Err(err) => {
                        tracing::warn!("Could not get Club Tree: {err}");
                        return Err(Cow::Owned(format!("Could not get Club Tree: {err}")));
                    },
                };
                let mut club = match repo.treebuilder(Some(&club)) {
                    Ok(c) => c,
                    Err(err) => {
                        tracing::warn!("Could not get Club TreeBuilder: {err}");
                        return Err(Cow::Owned(format!("Could not get Club TreeBuilder: {err}")));
                    },
                };
                add_files_top(files, repo, &mut club)?
            };
            let _ = match codes.insert(club_name, club_oid, 0o040000) {
                Ok(v) => v,
                Err(err) => {
                    tracing::warn!("Could not insert tree-oid to codes tree-builder: {err}");
                    return Err(Cow::Owned(format!("Could not insert tree-oid to codes tree-builder: {err}")));
                }
            };
            match codes.write() {
                Ok(v) => v,
                Err(err) => {
                    tracing::warn!("Could not write Codes-Tree to repo: {err}");
                    return Err(Cow::Owned(format!("Could not write Codes-Tree to repo: {err}")));
                }
            }
        };
        let _ = match builder.insert(CODES_NAME, codes_oid, 0o040000) {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!("Could not insert blob-oid to root tree-builder: {err}");
                return Err(Cow::Owned(format!("Could not insert blob-oid to root tree-builder: {err}")));
            }
        };
        Ok(())
    }
}
pub fn push_files(
    repo: &git2::Repository,
    target_branch_name: &str,
    club_name: &str,
    files: Vec<(&[u8], &str)>,
    commit_message: &str,
) -> Result<(), Cow<'static, str>>{
    let repo = &*repo;
    add_commit(
        repo,
        target_branch_name,
        add_files(files, club_name),
        commit_message
    )?;
    Ok(())
}


pub fn add_commit<'a>(
    repo: &'a git2::Repository,
    target_branch: &str,
    contents: impl FnOnce(&git2::Repository, &mut git2::TreeBuilder<'_>) -> Result<(), Cow<'static, str>>,
    message: &str,
) -> Result<(), Cow<'static, str>> {
    let mut remote = match repo.find_remote("origin") {
        Ok(remote) => remote,
        Err(err) => {
            tracing::warn!("Failed to find remote 'origin': {err}");
            return Err(Cow::Owned(format!("Failed to find remote 'origin': {err}")));
        },
    };
    match pull::pull(&mut remote){
        Ok(()) => {},
        Err(err) => {
            tracing::warn!("Failed to pull upstream 'origin': {err}");
            return Err(Cow::Owned(format!("Failed to pull upstream 'origin': {err}")));
        }
    }

    let branch = search_branch(repo, "origin/main").map_err(|err|Cow::Owned(format!("Could not find branch: {err}")))?;
    let commit = match branch.get().peel_to_commit() {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Error getting commit from branch-reference: {err}");
            return Err(Cow::Owned(format!("Error getting commit from branch-reference: {err}")));
        },
    };
    let tree = match branch.get().peel_to_tree() {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Error getting tree from branch-reference: {err}");
            return Err(Cow::Owned(format!("Error getting tree from branch-reference: {err}")));
        }
    };
    drop(branch);
    let mut treebuilder = match repo.treebuilder(Some(&tree)) {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Error creating treebuilder: {err}");
            return Err(Cow::Owned(format!("Error creating treebuilder: {err}")));
        }
    };
    contents(repo, &mut treebuilder)?;
    let tree = match treebuilder.write(){
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Error writing treebuilder: {err}");
            return Err(Cow::Owned(format!("Error writing treebuilder: {err}")));
        }
    };
    let tree = match repo.find_tree(tree){
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Error finding just written tree: {err}");
            return Err(Cow::Owned(format!("Error finding just written tree: {err}")));
        }
    };
    let signature = match git2::Signature::now("NeoLuma", "neoluma@c0d3m4513r.com") {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Error creating signature: {err}");
            return Err(Cow::Owned(format!("Error creating signature: {err}")));
        }
    };
    let commit_oid = match repo.commit(
        None,
        &signature,
        &signature,
        message,
        &tree,
        &[&commit]
    ) {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Error creating commit: {err}");
            return Err(Cow::Owned(format!("Error creating commit: {err}")));
        }
    };
    let commit = match repo.find_commit(commit_oid) {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Error finding just creating commit: {err}");
            return Err(Cow::Owned(format!("Error finding just creating commit: {err}")));
        }
    };
    match repo.branch(target_branch, &commit, true) {
        Ok(_) => (),
        Err(err) => {
            tracing::warn!("Error creating/overriding branch {target_branch}: {err}");
            return Err(Cow::Owned(format!("Error creating/overriding branch {target_branch}: {err}")));
        }
    };
    match push_commit_to_branch(repo, "origin", commit_oid, target_branch) {
        Ok(_) => Ok(()),
        Err(err) => Err(Cow::Owned(format!("Error pushing to branch-reference: {err}"))),
    }
}

fn push_commit_to_branch(
    repo: &git2::Repository,
    remote_name: &str,
    commit: git2::Oid,
    branch: &str,
) -> Result<(), Cow<'static, str>> {
    // Build refspec: <commit_sha>:refs/heads/<branch>
    let refspec = format!("+{}:refs/heads/{}", commit, branch);

    // Prepare callbacks/options (add auth etc. as needed)
    let mut callbacks = git2::RemoteCallbacks::new();
    auth::add_auth(&mut callbacks)?;
    callbacks.push_update_reference(|refname, status| {
        if let Some(err) = status {
            eprintln!("failed to update {refname}: {err}");
        }
        Ok(())
    });

    let mut push_opts = git2::PushOptions::new();
    push_opts.remote_callbacks(callbacks);

    // Find remote and push
    let mut remote = match repo.find_remote(remote_name){
        Ok(v) => v,
        Err(err) => return Err(Cow::Owned(format!("Error finding remote {remote_name}: {err}"))),
    };
    match remote.push(&[refspec], Some(&mut push_opts)) {
        Ok(()) => Ok(()),
        Err(err) => Err(Cow::Owned(format!("Error pushing to remote {remote_name}: {err}"))),
    }
}
