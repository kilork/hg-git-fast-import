# Mercurial to Git converter using git fast-import with multi repository import support

[![Linux build status](https://travis-ci.org/kilork/hg-git-fast-import.svg)](https://travis-ci.org/kilork/hg-git-fast-import)
[![Windows build status](https://ci.appveyor.com/api/projects/status/github/kilork/hg-git-fast-import?svg=true)](https://ci.appveyor.com/project/kilork/hg-git-fast-import)
[![Crates.io](https://img.shields.io/crates/v/hg-git-fast-import.svg)](https://crates.io/crates/hg-git-fast-import)
[![Packaging status](https://repology.org/badge/tiny-repos/hg-git-fast-import.svg)](https://repology.org/project/hg-git-fast-import/badges)
[![hg-git-fast-import](https://snapcraft.io//hg-git-fast-import/badge.svg)](https://snapcraft.io/hg-git-fast-import)

## Legal

Dual-licensed under `MIT` or the [UNLICENSE](http://unlicense.org/).

## Features

1. Import of single and multiple Mercurial repositories to Git repository.
1. Import of new revisions from previously imported Mercurial repositories to Git repository.
1. Tags.
1. Closed branches.
1. Verification of the end result with diff.

## Installation

With `cargo`:

    cargo install hg-git-fast-import

From source:

    git clone https://github.com/kilork/hg-git-fast-import.git
    cd hg-git-fast-import
    cargo install --path .

Prebuild release binaries:

[Download latest release](https://github.com/kilork/hg-git-fast-import/releases)

With Snap (*):

    sudo snap install hg-git-fast-import

(*) Limitations: Subprocesses are not allowed with strict snap package (hg and git), classic confinement is not requested at the moment, this means you can only export script which would be processed with git.

[![Get it from the Snap Store](https://snapcraft.io/static/images/badges/en/snap-store-black.svg)](https://snapcraft.io/hg-git-fast-import)


## Usage

**hg-git-fast-import** is a command-line utility, usage info can be access with --help argument:

```bash
$ hg-git-fast-import --help
hg-git-fast-import 1.3.6
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
hg-git-fast-import-single 1.3.6
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
hg-git-fast-import-multi 1.3.6
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
hg-git-fast-import-build-marks 1.3.6
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

See [single.toml](examples/single.toml).

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

See [multi.toml](examples/multi.toml).

### Authors list configuration example

```toml
'aaa 1' = 'Bbb <bbb@company.xyz>'
aaa = 'Bbb <bbb@company.xyz>'
ccc = 'Qqq <qqq@another.dom>'
'My <my_typo@wrong.xyz>' = 'My <my@normal.xyz>'
```

See [authors.toml](examples/authors.toml).

## Requirements

- Rust 1.32 or later (2018 edition)
- Git 2.19 (optional, if you use `single` mode without repo creation)
- Diff 2.8 (optional, if you do not use `--verify`)
- Mercurial 4.8 (optional, if you do not need delta load of revisions)
- Python 2.7 (optional, required for `Mercurial`)

## Docker support

To setup all dependencies can be a tricky task - it is possible to use [```docker```](https://www.docker.com/) for running ```hg-git-fast-import```.

### Docker installation

    git clone https://github.com/kilork/hg-git-fast-import.git
    cd hg-git-fast-import/docker
    ./build.sh

### Docker running

    docker run -it --rm kilork/hg-git-fast-import hg-git-fast-import --help

To mount current directory with repositories and run ```hg-git-fast-import``` command with docker one can use wrapper ```hg-git-fast-import/docker/run.sh```:

    cd hg-git-fast-import/docker
    ./run.sh

By default this will mount current directory to ```/repositories``` dir inside docker container. This can be overriden by usage of env variable:

    HG_GIT_FAST_IMPORT_VOLUME=~/sandbox:/sandbox ./run.sh single /sandbox/source_hg /sandbox/target_git
