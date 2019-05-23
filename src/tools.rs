use std::collections::HashMap;
use std::path::Path;

use hg_parser::{
    file_content, Changeset, FileType, ManifestEntryDetails, MercurialRepository, Revision,
    SharedMercurialRepository,
};
use log::info;

use crate::error::ErrorKind;
use crate::git::GitTargetRepository;
use crate::TargetRepositoryError;

/// Build or update marks.
///
/// Useful if target Git repository was updated.
pub fn build_marks<P: AsRef<Path>>(
    authors: Option<HashMap<String, String>>,
    hg_repo: P,
    git_repo: P,
    offset: Option<usize>,
) -> Result<(), ErrorKind> {
    let git_repo = GitTargetRepository::open(git_repo);
    let mut git_cmd = git_repo.git_cmd(&[
        "log",
        "--reflog",
        "--reverse",
        "--format=format:%H%n%at%n%an <%ae>%n%s",
    ]);
    let git_output = git_cmd.output()?;
    if !git_output.status.success() {
        return Err(ErrorKind::Target(TargetRepositoryError::GitFailure(
            git_output.status,
        )));
    }
    info!("{}", git_output.stdout.len());

    let hg_repo = MercurialRepository::open(hg_repo)?;
    Ok(())
}
