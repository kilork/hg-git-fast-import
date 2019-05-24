use std::collections::{BTreeMap, HashMap};

use crate::error::ErrorKind;
use crate::git::GitTargetRepository;
use crate::{read_file, TargetRepositoryError};
use hg_parser::{
    file_content, Changeset, FileType, ManifestEntryDetails, MercurialRepository, Revision,
    SharedMercurialRepository,
};
use log::info;
use std::io::Write;
use std::path::Path;


#[derive(Hash, PartialEq, Eq)]
struct RevisionHeader {
    user: String,
    date: usize,
    comment_first_line: Vec<u8>,
}

fn load_git_revlog_lines(stdout: &[u8]) -> HashMap<RevisionHeader, String> {
    let mut lines = stdout.split(|&x| x == b'\n');
    let mut result = HashMap::new();
    while let (Some(sha1), Some(date), Some(user), Some(comment)) =
        (lines.next(), lines.next(), lines.next(), lines.next())
    {
        let revision_header = RevisionHeader {
            user: String::from_utf8_lossy(&user).into(),
            date: String::from_utf8_lossy(&date).parse().unwrap(),
            comment_first_line: comment.into(),
        };
        result.insert(revision_header, String::from_utf8_lossy(&sha1).into());
    }
    result
}

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
        "--all",
        "--reverse",
        "--format=format:%H%n%at%n%an <%ae>%n%s",
    ]);
    let git_output = git_cmd.output()?;
    if !git_output.status.success() {
        return Err(ErrorKind::Target(TargetRepositoryError::GitFailure(
            git_output.status,
        )));
    }

    let git_repo_mapping = load_git_revlog_lines(&git_output.stdout);
    dbg!(git_repo_mapping.len());

    let hg_repo = MercurialRepository::open(hg_repo)?;

    let marks: BTreeMap<usize, String> =
        read_file(git_repo.path().join(".git/hg-git-fast-import.marks"))?
            .lines()
            .filter_map(|x| {
                let mut tokens = x.split_whitespace();
                if let (Some(mark), Some(sha1)) = (tokens.next(), tokens.next()) {
                    Some((mark[1..].parse().unwrap(), sha1.into()))
                } else {
                    None
                }
            })
            .collect();
    dbg!(marks.len());

    Ok(())
}
