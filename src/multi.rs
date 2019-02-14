use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use log::{debug, info};

use super::{config, MercurialRepo, RepositorySavedState, TargetRepository};
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
    env: &config::Environment,
    config_filename: P,
    multi_config: &config::MultiConfig,
) -> Result<(), ErrorKind> {
    let config_path = config_filename.as_ref().parent();

    for x in &multi_config.repositories {
        let path_hg = construct_path(&config_path, &x.path_hg);

        info!("Reading repo: {:?}", x.path_hg);
        let mercurial_repo = match MercurialRepo::open(&path_hg, &x.config, env) {
            Ok(repo) => repo,
            Err(ErrorKind::HgParserFailure(fail)) => {
                panic!("Cannot open {:?}: {:?}", path_hg, fail)
            }
            Err(other) => panic!("Cannot open {:?}: {:?}", path_hg, other),
        };

        info!("Verifying heads in repository {:?}", x.path_hg);
        if !mercurial_repo
            .verify_heads(x.config.allow_unnamed_heads)
            .unwrap()
        {
            return Err(ErrorKind::VerifyFailure("Verify heads failed".into()));
        }

        let tip = mercurial_repo.changelog_len().unwrap();

        let max = if let Some(limit_high) = x.config.limit_high {
            tip.min(limit_high)
        } else {
            tip
        };

        let offset = x.config.offset.unwrap_or(0);

        let path_git = construct_path(&config_path, &x.path_git);

        let mut git_repo = GitTargetRepository::open(path_git);

        let mut c: usize = 0;
        {
            let (output, saved_state) = git_repo.init().unwrap();

            let min = if let Some(saved_state) = saved_state.as_ref() {
                match saved_state {
                    RepositorySavedState::OffsetedRevision(rev) => rev - offset,
                }
            } else {
                0
            };

            let mut brmap = x
                .config
                .branches
                .as_ref()
                .map(|x| x.clone())
                .unwrap_or_else(|| HashMap::new());

            info!(
                "repo: {:?} min: {} max: {} offset: {:?}",
                x.path_hg, min, max, x.config.offset
            );
            info!("Exporting commits from {}", min);

            for mut changeset in mercurial_repo.range(min..max) {
                c = mercurial_repo.export_commit(&mut changeset, max, c, &mut brmap, output)?;
            }

            c = mercurial_repo.export_tags(min..max, c, output)?;
        }
        info!("Issued {} commands", c);
        info!("Saving state...");
        git_repo
            .save_state(RepositorySavedState::OffsetedRevision(max + offset))
            .unwrap();

        git_repo.finish().unwrap();

        if verify {
            git_repo
                .verify(
                    mercurial_repo.path().to_str().unwrap(),
                    x.config.path_prefix.as_ref().map(|x| &x[..]),
                )
                .unwrap();
        }
    }

    Ok(())
}
