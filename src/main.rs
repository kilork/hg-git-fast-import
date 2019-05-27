use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::Path;
use std::time::Instant;

use indicatif::HumanDuration;
use log::info;
use simplelog::{Config, LevelFilter, WriteLogger};

use structopt::StructOpt;

use hg_git_fast_import::config::RepositoryConfig;
use hg_git_fast_import::env::Environment;
use hg_git_fast_import::git::{GitTargetRepository, StdoutTargetRepository};
use hg_git_fast_import::{multi::multi2git, read_file, single::hg2git, tools::build_marks};

mod cli;

use self::cli::{
    Cli::{self, *},
    Common,
};

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
            limit_high,
            common,
        } => {
            if let Some(log) = common.log.as_ref() {
                setup_logger(log)
            }

            let env = load_environment(&common);

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
                    common.verify,
                    common.git_active_branches,
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
                    common.verify,
                    common.git_active_branches,
                    &mut stdout_target,
                    &env,
                    &repository_config,
                )
                .unwrap();
            }
            info!("Import done");
            if !common.cron {
                eprintln!(
                    "Finished. Time elapsed: {}",
                    HumanDuration(start_time.elapsed())
                );
            }
        }
        Multi { config, common } => {
            if let Some(log) = common.log.as_ref() {
                setup_logger(log)
            }

            let env = load_environment(&common);

            info!("Loading config");
            let config_str = read_file(&config).unwrap();
            let multi_config = toml::from_str(&config_str).unwrap();
            info!("Config loaded");
            multi2git(
                common.verify,
                common.git_active_branches,
                &env,
                &config,
                &multi_config,
            )
            .unwrap();
            info!("Import done");
            if !common.cron {
                eprintln!(
                    "Finished. Time elapsed: {}",
                    HumanDuration(start_time.elapsed())
                );
            }
        }
        BuildMarks { args } => {
            build_marks(
                args.authors.as_ref().map(load_authors),
                args.hg_repo,
                args.git_repo,
                args.offset,
                !args.no_backup,
            )
            .unwrap();
        }
    }
}

fn setup_logger(log: impl AsRef<Path>) {
    WriteLogger::init(
        LevelFilter::Info,
        Config::default(),
        File::create(log).unwrap(),
    )
    .unwrap();
}

fn load_environment(common: &Common) -> Environment {
    Environment {
        no_clean_closed_branches: common.no_clean_closed_branches,
        authors: common.authors.as_ref().map(load_authors),
        clean: common.clean,
        cron: common.cron,
        target_push: common.target_push,
        target_pull: common.target_pull,
        source_pull: common.source_pull,
        fix_wrong_branchname: common.fix_wrong_branchname,
    }
}

fn load_authors(p: impl AsRef<Path>) -> HashMap<String, String> {
    info!("Loading authors");
    let authors_str = read_file(p).unwrap();
    let authors = toml::from_str(&authors_str).unwrap();
    info!("Authors list loaded");
    authors
}
