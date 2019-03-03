use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use log::{debug, info};

use super::{config, env, MercurialRepo, RepositorySavedState, TargetRepository};
use crate::error::ErrorKind;
use crate::git::GitTargetRepository;

fn construct_path<P: AsRef<Path>>(config_path: &Option<P>, target: P) -> PathBuf {
    let target = target.as_ref();
    if target.is_absolute() {
        target.into()
    } else {
        config_path
            .as_ref()
            .map(|c| c.as_ref().join(target))
            .unwrap_or_else(|| target.into())
    }
}

pub fn multi2git<P: AsRef<Path>>(
    verify: bool,
    git_active_branches: Option<usize>,
    env: &env::Environment,
    config_filename: P,
    multi_config: &config::MultiConfig,
) -> Result<(), ErrorKind> {
    debug!("Config: {:?}", multi_config);
    debug!("Environment: {:?}", env);

    let config_path = config_filename.as_ref().parent();

    for repo in &multi_config.repositories {
        export_repository(&config_path, repo, env, verify, git_active_branches)?;
    }

    let path_git = construct_path(&config_path, &multi_config.path_git);

    let git_repo = GitTargetRepository::open(&path_git);

    let new_rerository = !path_git.exists();

    let remotes = if new_rerository {
        git_repo.create_repo()?;
        HashSet::new()
    } else {
        git_repo.remote_list()?
    };

    let mut merge = HashMap::new();
    for repo in &multi_config.repositories {
        let alias = repo
            .alias
            .as_ref()
            .unwrap_or_else(|| repo.config.path_prefix.as_ref().unwrap());
        if !remotes.contains(alias) {
            git_repo.remote_add(
                alias,
                construct_path(&config_path, &repo.path_git)
                    .canonicalize()?
                    .to_str()
                    .unwrap(),
            )?;
        }
        if let Some(merged_branches) = &repo.merged_branches {
            for (branch_to, branch_from) in merged_branches {
                merge
                    .entry(branch_to)
                    .or_insert_with(Vec::new)
                    .push(format!("{}/{}", alias, branch_from));
            }
        }
    }

    git_repo.fetch_all()?;

    for (branch_to, branches_from) in merge {
        git_repo.checkout(branch_to)?;

        if new_rerository {
            for branch_from in branches_from {
                git_repo.merge_unrelated(&[branch_from.as_ref()])?;
            }
        } else {
            let branches_from_str: Vec<_> = branches_from.iter().map(|x| x.as_ref()).collect();
            git_repo.merge_unrelated(&branches_from_str)?;
        }
    }

    Ok(())
}

fn export_repository(
    config_path: &Option<&Path>,
    repo: &config::PathRepositoryConfig,
    env: &env::Environment,
    verify: bool,
    git_active_branches: Option<usize>,
) -> Result<(), ErrorKind> {
    let path_hg = construct_path(&config_path, &repo.path_hg);

    info!("Reading repo: {:?}", repo.path_hg);
    let mercurial_repo = match MercurialRepo::open(&path_hg, &repo.config, env) {
        Ok(repo) => repo,
        Err(ErrorKind::HgParserFailure(fail)) => panic!("Cannot open {:?}: {:?}", path_hg, fail),
        Err(other) => panic!("Cannot open {:?}: {:?}", path_hg, other),
    };

    info!("Verifying heads in repository {:?}", repo.path_hg);
    if !mercurial_repo.verify_heads(repo.config.allow_unnamed_heads)? {
        return Err(ErrorKind::VerifyFailure("Verify heads failed".into()));
    }

    let tip = mercurial_repo.changelog_len()?;

    let to = if let Some(limit_high) = repo.config.limit_high {
        tip.min(limit_high)
    } else {
        tip
    };

    let offset = repo.config.offset.unwrap_or(0);

    let path_git = construct_path(&config_path, &repo.path_git);

    let mut git_repo = GitTargetRepository::open(path_git);

    let mut errors = None;
    let mut counter: usize = 0;
    let from_tag =
        {
            let (output, saved_state) = git_repo.start_import(git_active_branches, env.clean)?;

            let (from, from_tag) = if let Some(saved_state) = saved_state.as_ref() {
                match saved_state {
                    RepositorySavedState::OffsetedRevision(rev, from_tag) => {
                        (rev - offset, from_tag - offset)
                    }
                }
            } else {
                (0, 0)
            };

            let mut brmap = repo.config.branches.clone().unwrap_or_else(HashMap::new);

            info!(
                "Exporting commits from repo: {:?} from {} to {} offset {:?}",
                repo.path_hg, from, to, repo.config.offset
            );

            let start = Instant::now();
            let bar = ProgressBar::new((to - from) as u64);
            bar.set_style(ProgressStyle::default_bar().template(
                "{spinner:.green}[{elapsed_precise}] [{wide_bar:.cyan/blue}] {msg} ({eta})",
            ));
            for mut changeset in mercurial_repo.range(from..to) {
                bar.inc(1);
                bar.set_message(&format!("{:6}/{}", changeset.revision.0, to));
                match mercurial_repo.export_commit(&mut changeset, counter, &mut brmap, output) {
                    Ok(progress) => counter = progress,
                    x => {
                        errors = Some((x, changeset.revision.0));
                        break;
                    }
                }
            }

            if errors.is_none() {
                bar.finish_with_message(&format!(
                    "Repository {} [{};{}). Elapsed: {}",
                    repo.path_git.to_str().unwrap(),
                    from,
                    to,
                    HumanDuration(start.elapsed())
                ));

                counter = mercurial_repo.export_tags(from_tag..to, counter, output)?;
            }
            from_tag
        };

    if let Some((error, at)) = errors {
        if at > 0 {
            let at = at as usize;
            eprintln!("Import failed at {}", at);
            info!("Saving last success state at {}...", at);
            git_repo.save_state(RepositorySavedState::OffsetedRevision(
                at + offset,
                from_tag + offset,
            ))?;
        }
        error?;
    }

    info!("Issued {} commands", counter);

    info!("Saving state...");
    git_repo.save_state(RepositorySavedState::OffsetedRevision(
        to + offset,
        to + offset,
    ))?;

    git_repo.finish()?;

    if verify {
        git_repo.verify(
            mercurial_repo.path().to_str().unwrap(),
            repo.config.path_prefix.as_ref().map(|x| &x[..]),
        )?;
    }

    Ok(())
}
