# Mercurial to Git converter using git fast-import with multi repository import support

## Legal

Dual-licensed under `MIT` or the [UNLICENSE](http://unlicense.org/).

## Features

1. Import of single and multiple Mercurial repositories to Git repository.
1. Import of new revisions from previously imported Mercurial repositories to Git repository.
1. Tags.
1. Closed branches.
1. Verification of the end result with diff.

## Installation

    git clone https://github.com/kilork/hg-git-fast-import.git
    cd hg-git-fast-import
    cargo install --path .

## Usage

**hg-git-fast-import** is a command-line utility, usage info can be access with --help argument:

    $ hg-git-fast-import --help
    hg-git-fast-import 0.1.0
    Alexander Korolev <kilork@yandex.ru>
    A utility to import single and multiple Mercurial repositories to Git.

    USAGE:
        hg-git-fast-import <SUBCOMMAND>

    FLAGS:
        -h, --help       Prints help information
        -V, --version    Prints version information

    SUBCOMMANDS:
        help      Prints this message or the help of the given subcommand(s)
        multi     Exports multiple Mercurial repositories to single Git repo in fast-import compatible format
        single    Exports single Mercurial repository to Git fast-import compatible format

Import of single repository:

    $ hg-git-fast-import single --help
    hg-git-fast-import-single 0.1.0
    Alexander Korolev <kilork@yandex.ru>
    Exports single Mercurial repository to Git fast-import compatible format

    USAGE:
        hg-git-fast-import single [FLAGS] [OPTIONS] <hg_repo> [git_repo]

    FLAGS:
        -h, --help                        Prints help information
            --no-clean-closed-branches    Do not clean closed Mercurial branches.
        -V, --version                     Prints version information
            --verify                      Compares resulting Git repo with Mercurial.

    OPTIONS:
        -a, --authors <authors>    Authors remapping in toml format.
        -c, --config <config>      Repository configuration in toml format.

    ARGS:
        <hg_repo>     The Mercurial repo for import to git
        <git_repo>    The Git repo to import to. Creates repo if it does not exist. Otherwise saved state must exist.

Import of multiple repositories:

    $ hg-git-fast-import multi --help
    hg-git-fast-import-multi 0.1.0
    Alexander Korolev <kilork@yandex.ru>
    Exports multiple Mercurial repositories to single Git repo in fast-import compatible format

    USAGE:
        hg-git-fast-import multi [FLAGS] [OPTIONS] --config <config>

    FLAGS:
        -h, --help                        Prints help information
            --no-clean-closed-branches    Do not clean closed Mercurial branches.
        -V, --version                     Prints version information
            --verify                      Compares resulting Git repositories with Mercurial (only final state with
                                        subfolders).

    OPTIONS:
        -a, --authors <authors>    Authors remapping in toml format.
        -c, --config <config>      Repositories configuration in toml format.

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
