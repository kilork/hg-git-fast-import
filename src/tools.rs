use std::collections::{BTreeMap, HashMap};
use std::fs::{copy, File};
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use hg_parser::{ChangesetHeader, MercurialRepository};

use crate::error::ErrorKind;
use crate::git::GitTargetRepository;
use crate::{read_file, TargetRepositoryError};

use super::{to_str, to_string};

#[derive(Hash, PartialEq, Eq)]
struct RevisionHeader {
    user: String,
    date: usize,
}

/// Build or update marks.
///
/// Useful if target Git repository was updated.
pub fn build_marks<P: AsRef<Path>, S: ::std::hash::BuildHasher>(
    authors: Option<HashMap<String, String, S>>,
    hg_repo: P,
    git_repo: P,
    offset: Option<usize>,
    backup: bool,
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

    let (git_repo_mapping, revlog) = load_git_revlog_lines(&git_output.stdout);

    let hg_repo = MercurialRepository::open(hg_repo)?;

    let marks_file = git_repo.path().join(".git/hg-git-fast-import.marks");

    if backup && marks_file.exists() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let backup_marks_file = marks_file.with_extension(format!("marks.backup.{}", now));
        eprintln!("Save backup to {}", backup_marks_file.to_string_lossy());
        copy(&marks_file, backup_marks_file)?;
    }

    let mut build_marks = BuildMarks {
        git_repo,
        marks_file,
        hg_repo,
        git_repo_mapping,
        revlog,
    };

    build_marks.process(authors, offset)
}

struct BuildMarks<'a> {
    git_repo: GitTargetRepository<'a>,
    git_repo_mapping: HashMap<RevisionHeader, Vec<String>>,
    revlog: Vec<String>,
    marks_file: PathBuf,
    hg_repo: MercurialRepository,
}

impl<'a> BuildMarks<'a> {
    fn process<S: ::std::hash::BuildHasher>(
        &mut self,
        authors: Option<HashMap<String, String, S>>,
        offset: Option<usize>,
    ) -> Result<(), ErrorKind> {
        let authors = authors.as_ref();
        let offset = offset.unwrap_or_default() + 1;

        let mut marks = self.load_marks()?;

        for (index, rev) in self.hg_repo.header_iter().enumerate() {
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
                .and_then(|old_sha1| self.revlog_position(old_sha1));
            if old_sha1_pos.is_some() {
                continue;
            }

            let mapped_sha1_list = self.git_repo_mapping.get_mut(&revision_header);
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
                    sha1 = select_from_matching(
                        &self.git_repo,
                        &self.revlog,
                        &rev,
                        sha1s,
                        revision_mark,
                        index,
                        &revision_header,
                    )?;
                }
                if let Some(sha1) = sha1 {
                    if let Some(old_sha1) = marks.get(&revision_mark).cloned() {
                        if old_sha1 != sha1 {
                            let old_index = self.revlog_position(&old_sha1);
                            let new_index = self.revlog_position(&sha1);
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
                    "Cannot find Mercurial revision {} user {} timestamp {}",
                    index, revision_header.user, rev.time,
                );
            }
        }

        eprintln!("Writing updated marks");

        self.save_marks(marks)?;

        eprintln!("Done.");

        Ok(())
    }

    fn revlog_position(&self, sha1: &str) -> Option<usize> {
        self.revlog.iter().position(|x| x == sha1)
    }

    fn load_marks(&self) -> Result<BTreeMap<usize, String>, ErrorKind> {
        if !self.marks_file.exists() {
            return Ok(BTreeMap::new());
        }
        Ok(read_file(&self.marks_file)?
            .lines()
            .filter_map(|x| {
                let mut tokens = x.split_whitespace();
                if let (Some(mark), Some(sha1)) = (tokens.next(), tokens.next()) {
                    Some((mark[1..].parse().unwrap(), sha1.into()))
                } else {
                    None
                }
            })
            .collect())
    }

    fn save_marks(&self, marks: BTreeMap<usize, String>) -> Result<(), ErrorKind> {
        let mut f = File::create(&self.marks_file)?;

        for (mark, sha1) in marks {
            writeln!(f, ":{} {}", mark, sha1)?;
        }

        Ok(())
    }
}

fn select_from_matching(
    git_repo: &GitTargetRepository,
    revlog: &[String],
    rev: &ChangesetHeader,
    sha1s: &mut Vec<String>,
    revision_mark: usize,
    index: usize,
    revision_header: &RevisionHeader,
) -> Result<Option<String>, ErrorKind> {
    let indexes = find_matching_sha1_index(&rev, sha1s, git_repo)?;

    if indexes.len() == 1 {
        let removed = sha1s.remove(indexes[0]);
        eprintln!("Selected {}", &removed);
        return Ok(Some(removed));
    }

    let mut select = dialoguer::Select::new();
    select.with_prompt(&format!(
        "Select sha1 to set mark :{} (pos: {}) {} {}",
        revision_mark, index, revision_header.user, revision_header.date
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
        return Ok(Some(removed));
    }

    Ok(None)
}

fn find_matching_sha1_index(
    rev: &ChangesetHeader,
    sha1s: &[String],
    git_repo: &GitTargetRepository,
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
