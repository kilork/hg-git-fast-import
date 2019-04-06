use std::fs::File;
use std::io;
use std::path::PathBuf;
use std::time::Instant;

use indicatif::HumanDuration;
use log::info;
use simplelog::{Config, LevelFilter, WriteLogger};

use structopt::StructOpt;

use hg_git_fast_import::config::RepositoryConfig;
use hg_git_fast_import::env::Environment;
use hg_git_fast_import::git::{GitTargetRepository, StdoutTargetRepository};
use hg_git_fast_import::{multi::multi2git, read_file, single::hg2git};

mod cli;

use self::cli::Cli::{self, *};

fn main() {
    let start_time = Instant::now();

    let cli = Cli::from_args();
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
            git_active_branches,
            log,
            clean,
            cron,
        } => {
            log.as_ref().map(setup_logger);

            let env = load_environment(&authors, no_clean_closed_branches, clean, cron);

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

                git_target_repository.set_env(&env);

                hg2git(
                    hg_repo,
                    verify,
                    git_active_branches,
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
                    verify,
                    git_active_branches,
                    &mut stdout_target,
                    &env,
                    &repository_config,
                )
                .unwrap();
            }
            info!("Import done");
            if !cron {
                eprintln!(
                    "Finished. Time elapsed: {}",
                    HumanDuration(start_time.elapsed())
                );
            }
        }
        Multi {
            config,
            authors,
            no_clean_closed_branches,
            verify,
            git_active_branches,
            log,
            clean,
            cron,
        } => {
            log.as_ref().map(setup_logger);

            let env = load_environment(&authors, no_clean_closed_branches, clean, cron);

            info!("Loading config");
            let config_str = read_file(&config).unwrap();
            let multi_config = toml::from_str(&config_str).unwrap();
            info!("Config loaded");
            multi2git(verify, git_active_branches, &env, &config, &multi_config).unwrap();
            info!("Import done");
            if !cron {
                eprintln!(
                    "Finished. Time elapsed: {}",
                    HumanDuration(start_time.elapsed())
                );
            }
        }
    }
}

fn setup_logger(log: &PathBuf) {
    WriteLogger::init(
        LevelFilter::Info,
        Config::default(),
        File::create(log).unwrap(),
    )
    .unwrap();
}

fn load_environment(
    authors: &Option<PathBuf>,
    no_clean_closed_branches: bool,
    clean: bool,
    cron: bool,
) -> Environment {
    Environment {
        no_clean_closed_branches,
        authors: authors.as_ref().map(|x| {
            info!("Loading authors");
            let authors_str = read_file(&x).unwrap();
            let authors = toml::from_str(&authors_str).unwrap();
            info!("Authors list loaded");
            authors
        }),
        clean,
        cron,
    }
}
