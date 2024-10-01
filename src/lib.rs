#[doc = include_str!("../README.md")]
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fs::File,
    io::{
        self,
        prelude::{Read, Write},
    },
    ops::Range,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

use lazy_static::lazy_static;
use regex::Regex;
use tracing::{info, trace};

use ordered_parallel_iterator::OrderedParallelIterator;

pub mod config;
pub mod env;
pub mod error;
pub mod git;
pub mod multi;
pub mod single;
pub mod tools;

use self::config::RepositorySavedState;
pub use error::ErrorKind;

use hg_parser::{
    file_content, Changeset, FileType, ManifestEntryDetails, MercurialRepository,
    MercurialRepositoryOptions, Revision, SharedMercurialRepository,
};

pub fn read_file(filename: impl AsRef<Path>) -> io::Result<String> {
    let mut file = File::open(filename)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

fn to_str(bytes: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

fn to_string(bytes: &[u8]) -> String {
    to_str(bytes).into()
}

#[derive(Debug, thiserror::Error)]
pub enum TargetRepositoryError {
    #[error("unknown")]
    Nope,
    #[error("is not a directory")]
    IsNotDir,
    #[error("saved state does not exist")]
    SavedStateDoesNotExist,
    #[error("cannot init repository {0}")]
    CannotInitRepo(ExitStatus),
    #[error("cannot configure repository {0}")]
    CannotConfigRepo(ExitStatus),
    #[error("import failed {0}")]
    ImportFailed(ExitStatus),
    #[error("git failure {0}: {1}")]
    GitFailure(ExitStatus, String),
    #[error("io error {0}")]
    IOError(std::io::Error),
    #[error("verification failed")]
    VerifyFail,
}

impl From<std::io::Error> for TargetRepositoryError {
    fn from(value: std::io::Error) -> Self {
        TargetRepositoryError::IOError(value)
    }
}

pub trait TargetRepository {
    fn start_import(
        &mut self,
        git_active_branches: Option<usize>,
        default_branch: Option<&str>,
    ) -> Result<(&mut dyn Write, Option<config::RepositorySavedState>, String), TargetRepositoryError>;

    fn finish(&mut self) -> Result<(), TargetRepositoryError>;

    fn verify(
        &self,
        _verified_repo: &str,
        _subfolder: Option<&str>,
    ) -> Result<(), TargetRepositoryError> {
        Ok(())
    }

    fn save_state(&self, _state: RepositorySavedState) -> Result<(), TargetRepositoryError> {
        Ok(())
    }

    fn get_saved_state(&self) -> Option<&RepositorySavedState> {
        None
    }

    fn remote_list(&self) -> Result<HashSet<String>, TargetRepositoryError> {
        unimplemented!();
    }

    fn remote_add(&self, _name: &str, _url: &str) -> Result<(), TargetRepositoryError> {
        unimplemented!();
    }

    fn checkout(&self, _branch: &str) -> Result<(), TargetRepositoryError> {
        unimplemented!();
    }

    fn fetch_all(&self) -> Result<(), TargetRepositoryError> {
        unimplemented!();
    }

    fn merge_unrelated(&self, _branches: &[&str]) -> Result<(), TargetRepositoryError> {
        unimplemented!();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SourceRepositoryError {
    #[error("pull fail {0}")]
    PullFail(String),
}

struct MercurialRepo<'a> {
    path: PathBuf,
    inner: SharedMercurialRepository,
    config: &'a config::RepositoryConfig,
    env: &'a env::Environment,
}

impl<'a> MercurialRepo<'a> {
    /// Open Mercurial repository.
    pub fn open<P: AsRef<Path>>(
        path: P,
        config: &'a config::RepositoryConfig,
        ignore_unknown_requirements: bool,
        env: &'a env::Environment,
    ) -> Result<MercurialRepo<'a>, ErrorKind> {
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            inner: SharedMercurialRepository::new(MercurialRepository::open_with_options(
                path,
                MercurialRepositoryOptions {
                    ignore_unknown_requirements,
                },
            )?),
            config,
            env,
        })
    }

    /// Open Mercurial repository with pull by `hg pull -u` command before import.
    /// Pull command triggered only if `env.source_pull` is `true`.
    pub fn open_with_pull<P: AsRef<Path>>(
        path: P,
        config: &'a config::RepositoryConfig,
        ignore_unknown_requirements: bool,
        env: &'a env::Environment,
    ) -> Result<MercurialRepo<'a>, ErrorKind> {
        if env.source_pull {
            let mut hg = Command::new("hg");
            hg.args(["pull", "-u"]);

            if env.cron {
                hg.arg("-q");
            }

            let status = hg.current_dir(path.as_ref()).status()?;
            if !status.success() {
                return Err(SourceRepositoryError::PullFail(format!(
                    "Cannot pull {}",
                    path.as_ref().to_str().unwrap()
                ))
                .into());
            }
        }

        Self::open(path, config, ignore_unknown_requirements, env)
    }

    fn path(&self) -> &Path {
        self.path.as_path()
    }

    fn verify_heads(&self, _allow_unnamed_heads: bool) -> Result<bool, ErrorKind> {
        Ok(true)
    }

    fn changelog_len(&self) -> Result<usize, ErrorKind> {
        Ok(self.inner.last_rev().0 as usize)
    }

    fn fixup_user(&self, user: &str) -> Result<String, ErrorKind> {
        if let Some(ref authors) = self.config.authors {
            if let Some(remap) = authors.get(user).cloned() {
                return Ok(remap);
            }
        }

        if let Some(ref authors) = self.env.authors {
            if let Some(remap) = authors.get(user).cloned() {
                return Ok(remap);
            }
        }

        lazy_static! {
            static ref RE: Regex = Regex::new("([^<]+) ?(<[^>]*>)$").unwrap();
        }

        let (name, email) = if let Some(caps) = RE.captures(user) {
            (
                caps.get(1).unwrap().as_str().trim_end(),
                caps.get(2).unwrap().as_str(),
            )
        } else {
            return Err(ErrorKind::WrongUser(user.to_string()));
        };

        Ok(format!("{} {}", name, email))
    }

    fn mark<R: Into<usize>>(&self, revision: R) -> usize {
        revision.into() + 1 + self.config.offset.unwrap_or(0)
    }

    fn range(&self, range: Range<usize>) -> OrderedParallelIterator<Changeset> {
        self.inner.par_range_iter(range.into())
    }

    fn export_commit(
        &self,
        changeset: &mut Changeset,
        count: usize,
        brmap: &mut HashMap<String, String>,
        output: &mut dyn Write,
        default_branch: &str,
    ) -> Result<usize, ErrorKind> {
        let header = &changeset.header;

        let user = self.fixup_user(std::str::from_utf8(&header.user)?)?;

        let mut branch = None;
        let mut closed = false;
        for (key, value) in &header.extra {
            if key == b"branch" {
                branch = Some(value.as_slice());
            }

            if key == b"close" && value == b"1" {
                closed = true;
            }
        }
        let branch: String =
            std::str::from_utf8(branch.unwrap_or(default_branch.as_bytes()))?.into();

        let branch = brmap.entry(branch.clone()).or_insert_with(|| {
            sanitize_branchname(
                &branch,
                if branch != default_branch || self.config.prefix_default_branch {
                    self.config.branch_prefix.as_ref()
                } else {
                    None
                },
                self.env.fix_wrong_branchname,
            )
        });

        let revision = changeset.revision;

        if header.p1.is_some() || header.p2.is_some() || revision != 0.into() {
            writeln!(output, "reset refs/heads/{}", branch)?;
        }
        let desc = String::from_utf8_lossy(&header.comment);

        let time = header.time.timestamp_secs();
        let timezone = header.time.tz_offset_secs();
        let tz = format!("{:+03}{:02}", -timezone / 3600, ((-timezone % 3600) / 60));

        writeln!(output, "commit refs/heads/{}", branch)?;
        let mark = self.mark(revision);
        writeln!(output, "mark :{}", mark)?;

        writeln!(output, "author {} {} {}", user, time, tz)?;
        writeln!(output, "committer {} {} {}", user, time, tz)?;
        writeln!(output, "data {}", desc.len() + 1)?;
        writeln!(output, "{}\n", desc)?;

        match (header.p1, header.p2) {
            (Some(p1), Some(p2)) => {
                writeln!(output, "from :{}", self.mark(p1))?;
                writeln!(output, "merge :{}", self.mark(p2))?;
            }
            (Some(p), None) | (None, Some(p)) => {
                writeln!(output, "from :{}", self.mark(p))?;
            }
            _ => (),
        }

        info!(
            "{} ({}) | {} | {} | {} | {}",
            mark, revision.0, branch, user, desc, header.time
        );

        if self.env.cron {
            eprintln!(
                "{} ({}) | {} | {} | {} | {}",
                mark, revision.0, branch, user, desc, header.time
            );
        }

        let prefix = strip_leading_slash(self.config.path_prefix.as_ref(), "");
        for file in &mut changeset.files {
            match (&mut file.data, &mut file.manifest_entry) {
                (None, None) => {
                    write!(output, "D {}", prefix)?;
                    output.write_all(&file.path)?;
                    writeln!(output)?;
                }
                (Some(data), Some(manifest_entry)) => {
                    write!(
                        output,
                        "M {} inline {}",
                        match manifest_entry.details {
                            ManifestEntryDetails::File(FileType::Symlink) => "120000",
                            ManifestEntryDetails::File(FileType::Executable) => "100755",
                            ManifestEntryDetails::Tree
                            | ManifestEntryDetails::File(FileType::Regular) => "100644",
                        },
                        prefix
                    )?;
                    output.write_all(&file.path)?;
                    let data = file_content(data);
                    writeln!(output, "\ndata {}", data.len())?;
                    output.write_all(data)?;
                }
                _ => {
                    return Err(ErrorKind::WrongFileData(
                        String::from_utf8_lossy(&file.path).into(),
                    ))
                }
            }
        }

        if closed {
            writeln!(output, "reset refs/tags/archive/{}", branch)?;
            writeln!(output, "from :{}\n", self.mark(revision))?;

            writeln!(output, "reset refs/heads/{}", branch)?;
            writeln!(output, "from 0000000000000000000000000000000000000000\n")?;
        }
        Ok(count + 1)
    }

    fn export_tags(
        &self,
        range: Range<usize>,
        mut count: usize,
        output: &mut dyn Write,
    ) -> Result<usize, ErrorKind> {
        info!("Exporting tags");
        for (revision, tag) in self
            .inner
            .tags()?
            .range(Revision::from(range.start as u32)..Revision::from(range.end as u32))
        {
            let tag = sanitize_name(&tag.name, self.config.tag_prefix.as_ref(), "tag");

            writeln!(output, "reset refs/tags/{}", tag).unwrap();
            writeln!(output, "from :{}", self.mark(*revision)).unwrap();
            writeln!(output).unwrap();
            count += 1;
        }
        Ok(count)
    }
}

fn strip_leading_slash(prefix: Option<&String>, x: &str) -> String {
    prefix.map_or_else(|| x.to_string(), |p| format!("{}/{}", p, x))
}

fn sanitize_branchname(name: &str, prefix: Option<&String>, fix_branch_name: bool) -> String {
    let branchname = sanitize_name(name, prefix, "branch");
    if !fix_branch_name {
        return branchname;
    }
    let mut result = String::new();
    let mut chars = branchname.chars().peekable();
    let mut last = None;
    while let Some(&c) = chars.peek() {
        if c != '/' {
            break;
        }
        result.push(c);
        last = chars.next();
    }
    while let Some(&c) = chars.peek() {
        let c = match c {
            '\0'..=' ' | '~' | '^' | ':' | '\\' => '-',
            '.' if last == Some('.') || last.is_none() => '-',
            c => c,
        };
        result.push(c);
        last = chars.next();
    }
    if result.ends_with('/') {
        result.remove(result.len() - 1);
        result.push('-');
    }
    if result.ends_with(".lock") {
        result.replace_range((result.len() - 5)..=(result.len() - 5), "-");
    }
    result
}

fn sanitize_name(name: &str, prefix: Option<&String>, what: &str) -> String {
    trace!("Sanitize {} '{}'", what, name);
    prefix.map_or_else(|| name.into(), |p| format!("{}{}", p, name))

    //TODO: git-check-ref-format
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_branchnames() {
        assert_eq!(&sanitize_branchname("normal", None, false), "normal");
        assert_eq!(&sanitize_branchname("normal", None, true), "normal");
        assert_eq!(&sanitize_branchname("////normal", None, true), "////normal");
        assert_eq!(
            &sanitize_branchname("with spaces  ", None, true),
            "with-spaces--"
        );
        assert_eq!(
            &sanitize_branchname("with spaces  ", Some(&"prefix-".into()), true),
            "prefix-with-spaces--"
        );
        assert_eq!(
            &sanitize_branchname(".dotatstart", None, true),
            "-dotatstart"
        );
        assert_eq!(
            &sanitize_branchname("dots.in.the.middle", None, true),
            "dots.in.the.middle"
        );
        assert_eq!(
            &sanitize_branchname("doubledots..", None, true),
            "doubledots.-"
        );
        assert_eq!(&sanitize_branchname("...", None, true), "---");
        assert_eq!(
            &sanitize_branchname("branch.lock", None, true),
            "branch-lock"
        );
        assert_eq!(&sanitize_branchname("//qqq//", None, true), "//qqq/-");
    }
}
