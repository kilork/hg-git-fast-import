use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::Path;
use std::time::Instant;

use exitfailure::ExitFailure;
use failure::{self, ResultExt};
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

fn main() -> Result<(), ExitFailure> {
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

            let env = load_environment(&common)?;

            let repository_config: Result<RepositoryConfig, failure::Error> = config.map_or_else(
                || Ok(RepositoryConfig::default()),
                |x| {
                    info!("Loading config");
                    let config_str = read_file(&x)
                        .with_context(|_| format!("Cannot read config {:?}", x))?;
                    let mut config: RepositoryConfig = toml::from_str(&config_str)
                        .with_context(|_| format!("Cannot parse config {:?}", x))?;
                    info!("Config loaded");
                    if limit_high.is_some() {
                        config.limit_high = limit_high;
                    }
                    Ok(config)
                },
            );
            let repository_config = repository_config?;
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
                )?;
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
                )?;
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

            let env = load_environment(&common)?;

            info!("Loading config");
            let config_str =
                read_file(&config).with_context(|_| format!("Cannot read config {:?}", config))?;
            let multi_config = toml::from_str(&config_str)
                .with_context(|_| format!("Cannot parse config from toml {:?}", config))?;
            info!("Config loaded");
            multi2git(
                common.verify,
                common.git_active_branches,
                &env,
                &config,
                &multi_config,
            )?;
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
                args.authors.as_ref().map(load_authors).transpose()?,
                args.hg_repo,
                args.git_repo,
                args.offset,
                !args.no_backup,
            )?;
        }
    }

    Ok(())
}

fn setup_logger(log: impl AsRef<Path>) {
    WriteLogger::init(
        LevelFilter::Info,
        Config::default(),
        File::create(log).unwrap(),
    )
    .unwrap();
}

fn load_environment(common: &Common) -> Result<Environment, failure::Error> {
    Ok(Environment {
        no_clean_closed_branches: common.no_clean_closed_branches,
        authors: common.authors.as_ref().map(load_authors).transpose()?,
        clean: common.clean,
        cron: common.cron,
        target_push: common.target_push,
        target_pull: common.target_pull,
        source_pull: common.source_pull,
        fix_wrong_branchname: common.fix_wrong_branchname,
    })
}

fn load_authors(p: impl AsRef<Path>) -> Result<HashMap<String, String>, failure::Error> {
    info!("Loading authors");
    let authors_str =
        read_file(&p).with_context(|_| format!("Cannot load authors {:?}", p.as_ref()))?;
    let authors = toml::from_str(&authors_str)
        .with_context(|_| format!("Cannot parse authors from toml {:?}", p.as_ref()))?;
    info!("Authors list loaded");
    Ok(authors)
}
