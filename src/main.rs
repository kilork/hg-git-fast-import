use std::io;
use std::path::PathBuf;

use log::{info, trace};

use structopt::StructOpt;

use hg_git_fast_import::config::{Environment, RepositoryConfig};
use hg_git_fast_import::git::{GitTargetRepository, StdoutTargetRepository};
use hg_git_fast_import::{single::hg2git, multi::multi2git, read_file};

use env_logger::{Builder, Env};
mod cli;

use self::cli::Cli::{self, *};

fn main() {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let cli = Cli::from_args();
    trace!("cli: {:?}", cli);
    match cli {
        Completions { shell } => {
            cli::Cli::clap().gen_completions_to(env!("CARGO_PKG_NAME"), shell, &mut io::stdout());
        }
        Single {
            hg_repo,
            git_repo,
            config,
            authors,
            no_clean_closed_branches,
            verify,
            limit_high,
        } => {
            let env = load_environment(&authors, no_clean_closed_branches);

            let repository_config = config.map_or_else(RepositoryConfig::default, |x| {
                info!("Loading config");
                let config_str = read_file(&x).unwrap();
                let mut config: RepositoryConfig = toml::from_str(&config_str).unwrap();
                info!("Config loaded");
                if limit_high.is_some() {
                    config.limit_high = limit_high;
                }
                config
            });
            if let Some(git_repo) = git_repo {
                let mut git_target_repository = GitTargetRepository::open(git_repo);
                hg2git(
                    hg_repo,
                    false,
                    verify,
                    &mut git_target_repository,
                    &env,
                    &repository_config,
                )
                .unwrap();
            } else {
                let stdout = std::io::stdout();
                let stdoutlock = stdout.lock();
                let mut stdout_target = StdoutTargetRepository::from(stdoutlock);
                hg2git(
                    hg_repo,
                    false,
                    verify,
                    &mut stdout_target,
                    &env,
                    &repository_config,
                )
                .unwrap();
            }
        }
        Multi {
            git_repo,
            config,
            authors,
            no_clean_closed_branches,
            verify,
        } => {
            let env = load_environment(&authors, no_clean_closed_branches);
            info!("Loading config");
            let config_str = read_file(&config).unwrap();
            let config = toml::from_str(&config_str).unwrap();
            info!("Config loaded");
            if let Some(git_repo) = git_repo {
                let mut git_target_repository = GitTargetRepository::open(git_repo);
                multi2git(false, verify, &mut git_target_repository, &env, &config).unwrap();
            } else {
                unimplemented!();
            }
        }
    }
}

fn load_environment(authors: &Option<PathBuf>, no_clean_closed_branches: bool) -> Environment {
    Environment {
        no_clean_closed_branches,
        authors: authors.as_ref().map(|x| {
            info!("Loading authors");
            let authors_str = read_file(&x).unwrap();
            let authors = toml::from_str(&authors_str).unwrap();
            info!("Authors list loaded");
            authors
        }),
    }
}
