use std::path::PathBuf;
use structopt::{self, StructOpt};

#[derive(Debug, StructOpt)]
pub enum Cli {
    /// Exports single Mercurial repository to Git fast-import compatible format
    #[structopt(name = "single")]
    Single {
        /// The Mercurial repo for import to git
        #[structopt(parse(from_os_str))]
        hg_repo: PathBuf,
        /// The Git repo to import to. Creates repo if it does not exist. Otherwise saved state must exist.
        #[structopt(parse(from_os_str))]
        git_repo: Option<PathBuf>,
        /// Repository configuration in toml format.
        #[structopt(parse(from_os_str), long, short)]
        config: Option<PathBuf>,
        /// Authors remapping in toml format.
        #[structopt(parse(from_os_str), long, short)]
        authors: Option<PathBuf>,
        /// Do not clean closed Mercurial branches.
        #[structopt(name = "no-clean-closed-branches", long)]
        no_clean_closed_branches: bool,
        /// Compares resulting Git repo with Mercurial.
        #[structopt(long)]
        verify: bool,
        /// Limit high revision to import.
        #[structopt(name = "limit-high", long)]
        limit_high: Option<usize>,
        /// Git maximum number of branches to maintain active at once.
        #[structopt(name = "git-active-branches", long)]
        git_active_branches: Option<usize>,
        /// Log file. If present - additional log info would be printed to this file.
        #[structopt(parse(from_os_str), long)]
        log: Option<PathBuf>,
    },
    /// Exports multiple Mercurial repositories to single Git repo in fast-import compatible format
    #[structopt(name = "multi")]
    Multi {
        /// Repositories configuration in toml format.
        #[structopt(parse(from_os_str), long, short)]
        config: PathBuf,
        /// Authors remapping in toml format.
        #[structopt(parse(from_os_str), long, short)]
        authors: Option<PathBuf>,
        /// Do not clean closed Mercurial branches.
        #[structopt(name = "no-clean-closed-branches", long)]
        no_clean_closed_branches: bool,
        /// Compares resulting Git repositories with Mercurial (only final state with subfolders).
        #[structopt(long)]
        verify: bool,
        /// Git maximum number of branches to maintain active at once.
        #[structopt(name = "git-active-branches", long)]
        git_active_branches: Option<usize>,
        /// Log file. If present - additional log info would be printed to this file.
        #[structopt(parse(from_os_str), long)]
        log: Option<PathBuf>,
    },
    /// Generates completion scripts for your shell
    #[structopt(
        name = "completions",
        raw(setting = "structopt::clap::AppSettings::Hidden")
    )]
    Completions {
        /// The shell to generate the script for
        #[structopt(raw(possible_values = r#"&["bash", "fish", "zsh"]"#))]
        shell: structopt::clap::Shell,
    },
}
