pub mod pull;
pub mod auth;
pub mod push;

use std::borrow::Cow;

fn search_branch<'a>(repo: &'a git2::Repository, target_branch_name: &str) -> Result<git2::Branch<'a>, Cow<'static, str>> {
    let branches = match repo.branches(None) {
        Ok(v) => v,
        Err(err) => return Err(Cow::Owned(format!("Error getting repo branches: {err}"))),
    };

    for branch in branches {
        let (branch, _) = match branch {
            Ok(branch) => branch,
            Err(err) => {
                tracing::error!("Error getting branch: {err}");
                continue;
            }
        };
        match branch.name_bytes() {
            Ok(branch_name) => match core::str::from_utf8(branch_name) {
                Ok(branch_name) => {
                    if target_branch_name == branch_name {
                        return Ok(branch);
                    }
                }
                Err(err) => {
                    tracing::debug!("Branch without utf-8 branch name: {err}");
                    continue;
                }
            }
            Err(err) => {
                tracing::info!("Cannot get branch-name: {err}");
                continue;
            }
        }
    }

    let head = match repo.head() {
        Ok(head) => head,
        Err(err) => return Err(Cow::Owned(format!("Error getting repo Head: {err}"))),
    };
    let commit = match head.peel_to_commit() {
        Ok(head) => head,
        Err(err) => return Err(Cow::Owned(format!("Error getting commit from Head-Reference: {err}"))),
    };
    match repo.branch(target_branch_name, &commit, false) {
        Ok(head) => Ok(head),
        Err(err) => Err(Cow::Owned(format!("Error getting commit from Head-Reference: {err}"))),
    }
}