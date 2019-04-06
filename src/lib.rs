use lazy_static::lazy_static;
use std::collections::HashSet;
use std::ops::Range;

use log::{info, trace};

use regex::Regex;

use ordered_parallel_iterator::OrderedParallelIterator;

use std::collections::HashMap;
use std::fs::File;
use std::io::{
    self,
    prelude::{Read, Write},
};
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitStatus;

pub mod config;
pub mod env;
pub mod error;
pub mod git;
pub mod multi;
pub mod single;

use self::config::RepositorySavedState;
pub use error::ErrorKind;

use hg_parser::{
    file_content, Changeset, FileType, ManifestEntryDetails, MercurialRepository, Revision,
    SharedMercurialRepository,
};

pub fn read_file(filename: &PathBuf) -> io::Result<String> {
    let mut file = File::open(filename)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

#[derive(Debug)]
pub enum TargetRepositoryError {
    Nope,
    IsNotDir,
    SavedStateDoesNotExist,
    CannotInitRepo(ExitStatus),
    CannotConfigRepo(ExitStatus),
    ImportFailed(ExitStatus),
    IOError(std::io::Error),
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
    ) -> Result<(&mut Write, Option<config::RepositorySavedState>), TargetRepositoryError>;

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

struct MercurialRepo<'a> {
    path: PathBuf,
    inner: SharedMercurialRepository,
    config: &'a config::RepositoryConfig,
    env: &'a env::Environment,
}

impl<'a> MercurialRepo<'a> {
    pub fn open<P: AsRef<Path>>(
        path: P,
        config: &'a config::RepositoryConfig,
        env: &'a env::Environment,
    ) -> Result<MercurialRepo<'a>, ErrorKind> {
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            inner: SharedMercurialRepository::new(MercurialRepository::open(path)?),
            config,
            env,
        })
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

    fn fixup_user(&self, user: &str) -> String {
        if let Some(ref authors) = self.config.authors {
            if let Some(remap) = authors.get(user) {
                return remap.clone();
            }
        }

        if let Some(ref authors) = self.env.authors {
            if let Some(remap) = authors.get(user) {
                return remap.clone();
            }
        }

        lazy_static! {
            static ref RE: Regex = Regex::new("([^<]+) ?(<[^>]*>)$").unwrap();
        }

        let (name, email) = if let Some(caps) = RE.captures(&user) {
            (
                caps.get(1).unwrap().as_str().trim_end(),
                caps.get(2).unwrap().as_str(),
            )
        } else {
            panic!("Wrong user: {}", user);
        };

        format!("{} {}", name, email)
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
        output: &mut Write,
    ) -> Result<usize, ErrorKind> {
        let header = &changeset.header;

        let user = self.fixup_user(std::str::from_utf8(&header.user)?);

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
        let branch: String = std::str::from_utf8(branch.unwrap_or_else(|| b"master"))?.into();

        let branch = brmap.entry(branch.clone()).or_insert_with(|| {
            sanitize_name(
                &branch,
                if branch != "master" || self.config.prefix_default_branch {
                    self.config.branch_prefix.as_ref()
                } else {
                    None
                },
                "branch",
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
            "{}({}) {} {} {} {}",
            mark, revision.0, branch, user, desc, header.time
        );

        if self.env.cron {
            eprintln!(
                "{}({}) {} {} {} {}",
                mark, revision.0, branch, user, desc, header.time
            );
        }

        let prefix = strip_leading_slash(self.config.path_prefix.as_ref(), &"".into());
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
                    let data = file_content(&data);
                    writeln!(output, "\ndata {}", data.len())?;
                    output.write_all(&data[..])?;
                }
                _ => panic!("Wrong file data!"),
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
        output: &mut Write,
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

fn strip_leading_slash(prefix: Option<&String>, x: &String) -> String {
    prefix.map_or_else(|| x.to_string(), |p| format!("{}/{}", p, x))
}

fn sanitize_name(name: &str, prefix: Option<&String>, what: &str) -> String {
    trace!("Sanitize {} '{}'", what, name);
    prefix.map_or_else(|| name.into(), |p| format!("{}{}", p, name))

    //TODO: git-check-ref-format
}
