# **hg-git-fast-import** - **mercurial** to **git** converter using **git fast-import** and written in **Rust Language**

## Legal

Dual-licensed under MIT or the [UNLICENSE](http://unlicense.org/).

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

## Requirements

- Rust 1.31
- Diff 2.8
- Git 2.19
- Mercurial 4.8
- Python 2.7

## Implementation details

**hg-git-fast-import** uses Python Mercurial libraries to access repository info and converts it to Git fast-import format. This is done by usage of [rust-python](https://github.com/dgrunwald/rust-cpython) Rust crate. It is slower than [original](https://github.com/frej/fast-export) Python implementation. Why not use just Python? I really like Python, but I also huge fun of Rust Language and found personally it is really nice fit for writing CLI apps. Especially knowing Rust has special [focus](https://www.rust-lang.org/what/cli) on CLI apps support.

In any case it is open source and you can volunteer and convert it back to python completely if performance is a blocker for you. In this section we describe technical details which can be important for such conversion.

Also, it is known - Mercurial is doing own Rust development, I expect to remove python parts in favor of either clean Rust implementation or usage of [Mercurial CommandServer](https://www.mercurial-scm.org/wiki/CommandServer).