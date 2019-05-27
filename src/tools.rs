use std::borrow::Cow;
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
}

fn to_str(bytes: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

fn to_string(bytes: &[u8]) -> String {
    to_str(bytes).into()
}

fn load_git_revlog_lines(stdout: &[u8]) -> HashMap<RevisionHeader, Vec<String>> {
    let mut lines = stdout.split(|&x| x == b'\n');
    let mut result: HashMap<RevisionHeader, Vec<String>> = HashMap::new();
    while let (Some(sha1), Some(date), Some(user)) = (lines.next(), lines.next(), lines.next()) {
        let revision_header = RevisionHeader {
            user: to_string(&user),
            date: to_str(&date).parse().unwrap(),
        };
        let sha1s = result.entry(revision_header).or_default();
        sha1s.push(to_string(&sha1));
    }
    result
}

/// Build or update marks.
///
/// Useful if target Git repository was updated.
pub fn build_marks<P: AsRef<Path>, S: ::std::hash::BuildHasher>(
    authors: Option<HashMap<String, String, S>>,
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
        "--format=format:%H%n%at%n%an <%ae>",
    ]);
    let git_output = git_cmd.output()?;
    if !git_output.status.success() {
        return Err(ErrorKind::Target(TargetRepositoryError::GitFailure(
            git_output.status,
        )));
    }

    let mut git_repo_mapping = load_git_revlog_lines(&git_output.stdout);

    let hg_repo = MercurialRepository::open(hg_repo)?;

    let mut marks: BTreeMap<usize, String> =
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

    let authors = authors.as_ref();
    let offset = offset.unwrap_or_default() + 1;
    for (index, rev) in hg_repo.header_iter().enumerate() {
        let user = to_string(&rev.user);
        let user = authors
            .and_then(|authors| authors.get(&user))
            .cloned()
            .unwrap_or(user);
        let date = rev.time.timestamp_secs() as usize;
        let revision_header = RevisionHeader { user, date };
        let revision_mark = index + offset;
        let mut mapped_sha1_list = git_repo_mapping.get_mut(&revision_header);
        if let Some(mut sha1s) = mapped_sha1_list {
            //if sha1s.len() == 1 {
                sha1s.remove(0);
                let sha1 = sha1s.iter().next().cloned().unwrap();
                if let Some(old_sha1) = marks.get(&revision_mark).cloned() {
                    if old_sha1 != sha1 {
                        eprintln!(
                            "{}: set {} from {} to {}",
                            index, revision_mark, old_sha1, sha1
                        );
                        marks.insert(revision_mark, sha1);
                    }
                } else {
                    marks.insert(revision_mark, sha1.clone());
                }
            /*} else {
                eprintln!("Found multiple ({}) sha1s for {}", sha1s.len(), revision_mark);
                for sha1 in sha1s {
                    let mut git_cmd = git_repo.git_cmd(&[
                        "show",
                        "-s",
                        "--format=%B.",
                        &sha1
                    ]);
                    let git_output = git_cmd.output()?;
                    if !git_output.status.success() {
                        return Err(ErrorKind::Target(TargetRepositoryError::GitFailure(
                            git_output.status,
                        )));
                    }
                    eprintln!("{}:{}", sha1, String::from_utf8_lossy(&git_output.stdout).trim_end());
                }
            }*/
        } else if mapped_sha1_list.is_none() {
            eprintln!(
                "Cannot find {} {} {}",
                index, revision_header.user, revision_header.date,
            );
        }
    }

    Ok(())
}
