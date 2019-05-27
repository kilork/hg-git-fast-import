use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::Write;
use std::path::Path;

use hg_parser::{ChangesetHeader, MercurialRepository};

use crate::error::ErrorKind;
use crate::git::GitTargetRepository;
use crate::{read_file, TargetRepositoryError};

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

fn load_git_revlog_lines(stdout: &[u8]) -> (HashMap<RevisionHeader, Vec<String>>, Vec<String>) {
    let mut lines = stdout.split(|&x| x == b'\n');
    let mut result: HashMap<RevisionHeader, Vec<String>> = HashMap::new();
    let mut revlog = vec![];
    while let (Some(sha1), Some(date), Some(user)) = (lines.next(), lines.next(), lines.next()) {
        let revision_header = RevisionHeader {
            user: to_string(&user),
            date: to_str(&date).parse().unwrap(),
        };
        let sha1s = result.entry(revision_header).or_default();
        let sha1 = to_string(&sha1);
        sha1s.push(sha1.clone());
        revlog.push(sha1);
    }
    (result, revlog)
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

    let (mut git_repo_mapping, revlog) = load_git_revlog_lines(&git_output.stdout);

    let hg_repo = MercurialRepository::open(hg_repo)?;

    let marks_file = git_repo.path().join(".git/hg-git-fast-import.marks");

    let mut marks: BTreeMap<usize, String> = read_file(&marks_file)?
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

        let old_sha1_pos = marks
            .get(&revision_mark)
            .cloned()
            .and_then(|old_sha1| revlog.iter().position(|x| x == &old_sha1));
        if old_sha1_pos.is_some() {
            continue;
        }

        let mapped_sha1_list = git_repo_mapping.get_mut(&revision_header);
        if let Some(sha1s) = mapped_sha1_list {
            let mut sha1 = None;
            if sha1s.len() == 1 {
                sha1 = Some(sha1s.remove(0));
            } else if !sha1s.is_empty() {
                eprintln!(
                    "Found multiple ({}) sha1s for mark :{}",
                    sha1s.len(),
                    revision_mark
                );
                let indexes = find_matching_sha1_index(&rev, sha1s, &git_repo)?;
                let mut select = dialoguer::Select::new();
                select.with_prompt(&format!(
                    "Select sha1 to set mark :{} (pos: {}) {} {}",
                    revision_mark, index, revision_header.user, date
                ));
                for &index in &indexes {
                    let sha1 = &sha1s[index];
                    let new_index = revlog.iter().position(|x| x == sha1);
                    select.item(&format!("{} ({:?})", sha1, new_index));
                }
                let index_selected = select.interact_opt()?.map(|index| indexes[index]);
                if let Some(index) = index_selected {
                    let removed = sha1s.remove(index);
                    eprintln!("Selected {}", &removed);
                    sha1 = Some(removed);
                }
            }
            if let Some(sha1) = sha1 {
                if let Some(old_sha1) = marks.get(&revision_mark).cloned() {
                    if old_sha1 != sha1 {
                        let old_index = revlog.iter().position(|x| x == &old_sha1);
                        let new_index = revlog.iter().position(|x| x == &sha1);
                        eprintln!(
                            "{}: set {} from {}({:?}) to {}({:?})",
                            index, revision_mark, old_sha1, old_index, sha1, new_index
                        );
                        marks.insert(revision_mark, sha1);
                    }
                } else {
                    marks.insert(revision_mark, sha1.clone());
                }
            }
        } else if mapped_sha1_list.is_none() {
            eprintln!(
                "Cannot find {} {} {}",
                index, revision_header.user, revision_header.date,
            );
        }
    }

    eprintln!("Writing updated marks");

    let mut f = File::create(marks_file)?;
    for (mark, sha1) in marks {
        writeln!(f, ":{} {}", mark, sha1)?;
    }

    eprintln!("Done");

    Ok(())
}

fn find_matching_sha1_index(
    rev: &ChangesetHeader,
    sha1s: &[String],
    git_repo: &GitTargetRepository<'_>,
) -> Result<Vec<usize>, ErrorKind> {
    let hg_commit_message = rev.comment.as_slice();
    let mut result = vec![];
    for (index, sha1) in sha1s.iter().enumerate() {
        let mut git_cmd = git_repo.git_cmd(&["show", "-s", "--format=.%B.", &sha1]);
        let git_output = git_cmd.output()?;
        if !git_output.status.success() {
            return Err(ErrorKind::Target(TargetRepositoryError::GitFailure(
                git_output.status,
            )));
        }
        let comment = &git_output.stdout;
        let mut from = 0;
        while comment[from] != b'.' {
            from += 1;
        }
        let mut to = comment.len() - 1;
        while comment[to] != b'.' {
            to -= 1;
        }
        let comment = &comment[from + 1..to - 1];
        if hg_commit_message == comment {
            result.push(index);
        }
    }
    Ok(result)
}
