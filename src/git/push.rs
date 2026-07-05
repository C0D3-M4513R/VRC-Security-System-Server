use std::borrow::Cow;
use crate::git::{auth, pull, search_branch};
pub fn has_file(
    name: &str,
    content: &[u8],
    repo: &git2::Repository,
    builder: &mut git2::TreeBuilder<'_>
) -> bool {
    let tree = match builder.get(name) {
        Ok(Some(v)) => v,
        Ok(None) => return false,
        Err(err) => {
            tracing::warn!("Could not get TreeEntry with given name '{name}': {err}");
            return false
        }
    };
    let object = match tree.to_object(repo) {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Could not get object of TreeEntry with name '{name}': {err}");
            return false;
        }
    };
    let blob = match object.peel_to_blob() {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("TreeEntry (with name '{name}') object did not point to a blob?: {err}");
            return false;
        }
    };
    blob.content() == content
}
pub fn add_files_top(
    files: Vec<(&[u8], &str)>,
    repo: &git2::Repository,
    builder: &mut git2::TreeBuilder<'_>
) -> Result<git2::Oid, Cow<'static, str>> {
    for (bytes, name) in files {
        if has_file(name, bytes, repo, builder) { continue; }
        let oid = match repo.blob(bytes) {
            Ok(oid) => oid,
            Err(err) => {
                tracing::warn!("Could not convert file content to oid for file {name}: {err}");
                return Err(Cow::Owned(format!("Could not convert file content to oid for file {name}: {err}")));
            }
        };
        let _ = match builder.insert(name, oid, FILEMODE_BLOB) {
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
pub fn change_folder<'a, T>(
    repo: &git2::Repository,
    builder: &mut git2::TreeBuilder,
    name: &str,
    create_folder: bool,
    filemode: i32,
    func: impl FnOnce(&git2::Repository, &mut git2::TreeBuilder<'_>) -> Result<T, Cow<'static, str>>,
) -> Result<T, Cow<'static, str>> {
    let mut inner_builder = match match builder.get(name) {
        Ok(Some(v)) => match match v.to_object(repo) {
            Ok(v) => v.peel_to_tree(),
            Err(err) => {
                tracing::warn!("Failed to convert Folder of {name} to git object: {err}");
                return Err(Cow::Owned(format!("Failed to convert Folder of {name} to git object: {err}")));
            }
        }{
            Ok(v) => repo.treebuilder(Some(&v)),
            Err(err) => {
                tracing::warn!("Failed to convert the object of the Folder of {name} to a git tree: {err}");
                return Err(Cow::Owned(format!("Failed to convert the object of the Folder of {name} to a git tree: {err}")));
            }
        },
        Ok(None) => if create_folder {
            repo.treebuilder(None)
        } else {
            tracing::warn!("{name} does not exist");
            return Err(Cow::Owned(format!("{name} does not exist")));
        },
        Err(err) => {
            tracing::warn!("Failed to query for {name} existence: {err}");
            return Err(Cow::Owned(format!("Failed to query for {name} existence: {err}")));
        }
    }{
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Failed to create a TreeBuilder for {name}: {err}");
            return Err(Cow::Owned(format!("Failed to create a TreeBuilder for {name}: {err}")));
        }
    };

    let out = match func(&repo, &mut inner_builder){
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Failed to populate subtree of {name}: {err}");
            return Err(Cow::Owned(format!("Failed to populate subtree of {name}: {err}")));
        }
    };

    let inner_builder = match inner_builder.write() {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Failed to write new tree for {name}: {err}");
            return Err(Cow::Owned(format!("Failed to write new tree for {name}: {err}")));
        }
    };

    match builder.insert(name, inner_builder, filemode){
        Ok(_) => {},
        Err(err) => {
            tracing::warn!("Failed to update the oid for {name}: {err}");
            return Err(Cow::Owned(format!("Failed to update the oid for {name}: {err}")));
        }
    }

    Ok(out)
}

const FILEMODE_TREE:i32 = 0o040000;
const FILEMODE_BLOB:i32 = 0o100644;

pub fn add_files(
    files: Vec<(&[u8], &str)>,
    club_name: &str,
) -> impl FnOnce(&git2::Repository, &mut git2::TreeBuilder<'_>) -> Result<(), Cow<'static, str>> {
    move |repo, builder|{
        const CODES_NAME:&str = "Codes";
        change_folder(repo, builder, CODES_NAME, true, FILEMODE_TREE, |repo, builder|{
           change_folder(repo, builder, club_name, true, FILEMODE_TREE, |repo, builder|{
               add_files_top(files, repo, builder)
           })
        })?;

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
