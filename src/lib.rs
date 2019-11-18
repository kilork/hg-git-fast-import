/*!
# Mercurial to Git converter using git fast-import with multi repository import support

```bash
hg-git-fast-import single hg_repo git_repo
```

## Features

1. Import of single and multiple Mercurial repositories to Git repository.
1. Import of new revisions from previously imported Mercurial repositories to Git repository.
1. Tags.
1. Closed branches.
1. Verification of the end result with diff.

## Installation

With `cargo`:

```bash
cargo install hg-git-fast-import
```

From source:

```bash
git clone https://github.com/kilork/hg-git-fast-import.git
cd hg-git-fast-import
cargo install --path .
```

## Usage

**hg-git-fast-import** is a command-line utility, usage info can be access with --help argument:

```bash
$ hg-git-fast-import --help
hg-git-fast-import 1.2.6
Alexander Korolev <kilork@yandex.ru>
A utility to import single and multiple Mercurial repositories to Git.

USAGE:
    hg-git-fast-import <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    build-marks    Rebuilds saved state of repo
    help           Prints this message or the help of the given subcommand(s)
    multi          Exports multiple Mercurial repositories to single Git repo in fast-import compatible format
    single         Exports single Mercurial repository to Git fast-import compatible format
```

Import of single repository:

```bash
$ hg-git-fast-import single --help
hg-git-fast-import-single 1.2.6
Alexander Korolev <kilork@yandex.ru>
Exports single Mercurial repository to Git fast-import compatible format

USAGE:
    hg-git-fast-import single [FLAGS] [OPTIONS] <hg_repo> [git_repo]

FLAGS:
        --clean                       Recreate Git repo before import if it exists.
        --cron                        Produce minimal output only if new revisions loaded or error happened.
        --fix-wrong-branch-names      Fix wrong Mercurial branch names (not compatible with git ref format).
    -h, --help                        Prints help information
        --no-clean-closed-branches    Do not clean closed Mercurial branches.
        --source-pull                 Pull source Mercurial repository before import.
        --target-pull                 Pull target Git repository before push.
        --target-push                 Push target Git repository after successful import.
    -V, --version                     Prints version information
        --verify                      Compares resulting Git repo with Mercurial.

OPTIONS:
    -a, --authors <authors>                            Authors remapping in toml format.
    -c, --config <config>                              Repository configuration in toml format.
        --git-active-branches <git-active-branches>    Git maximum number of branches to maintain active at once.
        --limit-high <limit-high>                      Limit high revision to import.
        --log <log>
            Log file. If present - additional log info would be printed to this file.


ARGS:
    <hg_repo>     The Mercurial repo for import to git
    <git_repo>    The Git repo to import to. Creates repo if it does not exist. Otherwise saved state must exist.
```

Import of multiple repositories:

```bash
$ hg-git-fast-import multi --help
hg-git-fast-import-multi 1.2.6
Alexander Korolev <kilork@yandex.ru>
Exports multiple Mercurial repositories to single Git repo in fast-import compatible format

USAGE:
    hg-git-fast-import multi [FLAGS] [OPTIONS] --config <config>

FLAGS:
        --clean                       Recreate Git repo before import if it exists.
        --cron                        Produce minimal output only if new revisions loaded or error happened.
        --fix-wrong-branch-names      Fix wrong Mercurial branch names (not compatible with git ref format).
    -h, --help                        Prints help information
        --no-clean-closed-branches    Do not clean closed Mercurial branches.
        --source-pull                 Pull source Mercurial repository before import.
        --target-pull                 Pull target Git repository before push.
        --target-push                 Push target Git repository after successful import.
    -V, --version                     Prints version information
        --verify                      Compares resulting Git repo with Mercurial.

OPTIONS:
    -a, --authors <authors>                            Authors remapping in toml format.
    -c, --config <config>                              Repositories configuration in toml format.
        --git-active-branches <git-active-branches>    Git maximum number of branches to maintain active at once.
        --log <log>
            Log file. If present - additional log info would be printed to this file.
```

Rebuild saved state of repo:

```bash
$ hg-git-fast-import build-marks --help
hg-git-fast-import-build-marks 1.2.6
Alexander Korolev <kilork@yandex.ru>
Rebuilds saved state of repo

USAGE:
    hg-git-fast-import build-marks [FLAGS] [OPTIONS] <hg_repo> <git_repo>

FLAGS:
    -h, --help         Prints help information
        --no-backup    Do not backup old marks.
    -V, --version      Prints version information

OPTIONS:
    -a, --authors <authors>    Authors remapping in toml format.
    -o, --offset <offset>      Offset for git fast-import marks in Git repository. Optional, default is 0.

ARGS:
    <hg_repo>     The Mercurial repo which was imported to git.
    <git_repo>    The Git repo to save state to. Existing saved state would be updated with actual state.
```

## Configuration syntax

For more advanced cases one may supply configuration in `toml` format.

### Single mode configuration example

```toml
# Allows to start import in of hanged heads in repository
# (currently has no effect, default value is true). Optional.
allow_unnamed_heads = true
# Offset for git fast-import marks in Git repository. Optional, default is 0.
offset = 1000
# Path prefix in target repository. If path_prefix = 'test',
# all files will be under test folder in target Git repository.
# Optional.
path_prefix = 'prefix1'
# Tag prefix in target repository. Optional.
tag_prefix = 'prefix2-'
# Branch prefix in target repository. Optional.
branch_prefix = 'prefix3-'
# By default master branch is not prefixed by branch_prefix.
# This behavior can be changed by specifying this as true.
# Optional.
prefix_default_branch = false

# Mapping between authors in Mercurial and authors in Git.
# Required mainly because of Git asks for particular format "Somename <email@address>".
# But also can be used to fix typos and etc.
[authors]
'aaa 1' = 'Bbb <bbb@company.xyz>'
aaa = 'Bbb <bbb@company.xyz>'
ccc = 'Qqq <qqq@another.dom>'
'My <my_typo@wrong.xyz>' = 'My <my@normal.xyz>'

# Mapping between branches in Mercurial and branches in Git.
# Required mainly because Git does not allow some characters,
# which allowed in Mercurial, for example - spaces.
# Branches taken from mapping will not have branch_prefix,
# so it must be added to mapped values.
[branches]
'branch in hg' = 'branch-in-git'
'anotherhg' = 'othergit'
```

With `authors` and `branches` subsections one can rename authors and branches during import. Offset creates marks in Git repository. Can be useful if all marks files from imported repositories planned to be analyzed together. `allow_unnamed_heads` allows to start import in case of hanged heads in repository, currently this feature has no effect.

### Multi mode configuration example

```toml
# Path to target git repository.
path_git = "000_git"

# This is subsection with list of repositories to be aggregated into single repo.
# Each subsection start like this (see toml format for arrays).
[[repositories]]
# Mercurial repository path.
path_hg = "001_hg"
# Child Git repository path.
path_git = "001_git"

# Child repository configuration for 001_hg/001_git.
# Fields are the same as on root level in single mode configuration.
[repositories.config]
allow_unnamed_heads = true
offset = 1000
path_prefix = 'prefix1'
tag_prefix = 'prefix2-'
branch_prefix = 'prefix3-'
prefix_default_branch = true

# Same as authors section in single mode, but for this child repository.
[repositories.config.authors]
'aaa' = 'Bbb <bbb@company.xyz>'

# Same as branches section in single mode, but for this child repository.
[repositories.config.branches]
'branch1' = 'branch2'

# This sections specify to which branches would be merged migrated
# branches from this child Git repository.
[repositories.merged_branches]
branch_in_git = 'branch2'
# Explanation: in this case branch_in_git will be a branch in 000_git repo
# and it will contain branch2 merged from remote child repository.

# This is second child repository.
[[repositories]]
# Here we can also specify alias, this field used to add reference in target 000_git repository.
# Otherwise path_prefix is used from config section.
alias = "another_002"
path_hg = "002_hg"
path_git = "002_git"

[repositories.merged_branches]
branch_in_git = 'branch_in_hg'
# Actually this branch_in_hg is from second migrated Git repository.
# Interesting to note - both child repository branches are merged
# into single branch_in_git branch in target 000_git repository.
```

Each of child repositories will be imported in corresponding `path_git` from configuration, then single repository from top level `path_git` will reference child repositories as `remote`. For remote name either `alias` either `path_prefix` is taken.

### Authors list configuration example

```toml
'aaa 1' = 'Bbb <bbb@company.xyz>'
aaa = 'Bbb <bbb@company.xyz>'
ccc = 'Qqq <qqq@another.dom>'
'My <my_typo@wrong.xyz>' = 'My <my@normal.xyz>'
```

## Requirements

- Rust 1.32 or later (2018 edition)
- Git 2.19 (optional, if you use `single` mode without repo creation)
- Diff 2.8 (optional, if you do not use `--verify`)
- Mercurial 4.8 (optional, if you do not need delta load of revisions)
- Python 2.7 (optional, required for `Mercurial`)

## Docker support

To setup all dependencies can be a tricky task - it is possible to use [```docker```](https://www.docker.com/) for running ```hg-git-fast-import```.

### Docker installation

```bash
git clone https://github.com/kilork/hg-git-fast-import.git
cd hg-git-fast-import/docker
./build.sh
```

### Docker running

```bash
docker run -it --rm kilork/hg-git-fast-import hg-git-fast-import --help
```

To mount current directory with repositories and run ```hg-git-fast-import``` command with docker one can use wrapper ```hg-git-fast-import/docker/run.sh```:

```bash
cd hg-git-fast-import/docker
./run.sh
```

By default this will mount current directory to ```/repositories``` dir inside docker container. This can be overriden by usage of env variable:

```bash
HG_GIT_FAST_IMPORT_VOLUME=~/sandbox:/sandbox ./run.sh single /sandbox/source_hg /sandbox/target_git
```

 */

use lazy_static::lazy_static;
use std::borrow::Cow;
use std::collections::HashSet;
use std::ops::Range;
use std::process::Command;

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

use failure::Fail;

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
    file_content, Changeset, FileType, ManifestEntryDetails, MercurialRepository, Revision,
    SharedMercurialRepository,
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

#[derive(Debug, Fail)]
pub enum TargetRepositoryError {
    #[fail(display = "unknown")]
    Nope,
    #[fail(display = "is not a directory")]
    IsNotDir,
    #[fail(display = "saved state does not exist")]
    SavedStateDoesNotExist,
    #[fail(display = "cannot init repository {}", _0)]
    CannotInitRepo(ExitStatus),
    #[fail(display = "cannot configure repository {}", _0)]
    CannotConfigRepo(ExitStatus),
    #[fail(display = "import failed {}", _0)]
    ImportFailed(ExitStatus),
    #[fail(display = "git failure {}", _0)]
    GitFailure(ExitStatus),
    #[fail(display = "io error {}", _0)]
    IOError(std::io::Error),
    #[fail(display = "verification failed")]
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
    ) -> Result<(&mut dyn Write, Option<config::RepositorySavedState>), TargetRepositoryError>;

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

#[derive(Debug, Fail)]
pub enum SourceRepositoryError {
    #[fail(display = "pull fail {}", _0)]
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
        env: &'a env::Environment,
    ) -> Result<MercurialRepo<'a>, ErrorKind> {
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            inner: SharedMercurialRepository::new(MercurialRepository::open(path)?),
            config,
            env,
        })
    }

    /// Open Mercurial repository with pull by `hg pull -u` command before import.
    /// Pull command triggered only if `env.source_pull` is `true`.
    pub fn open_with_pull<P: AsRef<Path>>(
        path: P,
        config: &'a config::RepositoryConfig,
        env: &'a env::Environment,
    ) -> Result<MercurialRepo<'a>, ErrorKind> {
        if env.source_pull {
            let mut hg = Command::new("hg");
            hg.args(&["pull", "-u"]);

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

        Self::open(path, config, env)
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

        let (name, email) = if let Some(caps) = RE.captures(&user) {
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
        let branch: String = std::str::from_utf8(branch.unwrap_or_else(|| b"master"))?.into();

        let branch = brmap.entry(branch.clone()).or_insert_with(|| {
            sanitize_branchname(
                &branch,
                if branch != "master" || self.config.prefix_default_branch {
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
            '.' if last == Some('.') || last == None => '-',
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
