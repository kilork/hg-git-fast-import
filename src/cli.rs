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
        /// Limit high revision to import.
        #[structopt(name = "limit-high", long)]
        limit_high: Option<usize>,
        #[structopt(flatten)]
        common: Common,
    },
    /// Exports multiple Mercurial repositories to single Git repo in fast-import compatible format
    #[structopt(name = "multi")]
    Multi {
        /// Repositories configuration in toml format.
        #[structopt(parse(from_os_str), long, short)]
        config: PathBuf,
        #[structopt(flatten)]
        common: Common,
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

#[derive(Debug, StructOpt)]
pub struct Common {
    /// Authors remapping in toml format.
    #[structopt(parse(from_os_str), long, short)]
    pub authors: Option<PathBuf>,
    /// Do not clean closed Mercurial branches.
    #[structopt(name = "no-clean-closed-branches", long)]
    pub no_clean_closed_branches: bool,
    /// Compares resulting Git repo with Mercurial.
    #[structopt(long)]
    pub verify: bool,
    /// Git maximum number of branches to maintain active at once.
    #[structopt(name = "git-active-branches", long)]
    pub git_active_branches: Option<usize>,
    /// Log file. If present - additional log info would be printed to this file.
    #[structopt(parse(from_os_str), long)]
    pub log: Option<PathBuf>,
    /// Recreate Git repo before import if it exists.
    #[structopt(long)]
    pub clean: bool,
    /// Produce minimal output only if new revisions loaded or error happened.
    #[structopt(long)]
    pub cron: bool,
    /// Push target Git repository after successful import.
    #[structopt(name = "target-push", long)]
    pub target_push: bool,
    /// Pull target Git repository before push.
    #[structopt(name = "target-pull", long)]
    pub target_pull: bool,
    /// Pull source Mercurial repository before import.
    #[structopt(name = "source-pull", long)]
    pub source_pull: bool,
    /// Fix wrong Mercurial branch names (not compatible with git ref format).
    #[structopt(name = "fix-wrong-branch-names", long)]
    pub fix_wrong_branchname: bool,
}
