use std::collections::HashMap;
use std::path::Path;

use log::{debug, info};

use super::{config, MercurialRepo, RepositorySavedState, TargetRepository};
use crate::error::ErrorKind;
use crate::git::GitTargetRepository;
use hg_parser::{Changeset, ChangesetIter};

struct ImportingRepository<'a> {
    min: usize,
    max: usize,
    brmap: HashMap<String, String>,
    heads: HashMap<String, usize>,
    path_config: &'a config::PathRepositoryConfig,
}

pub fn multi2git<P: AsRef<Path>>(
    verify: bool,
    env: &config::Environment,
    config_filename: P,
    multi_config: &config::MultiConfig,
) -> Result<(), ErrorKind> {
    let config_path = config_filename.as_ref().parent();

    for x in &multi_config.repositories {
        let path = if x.path_hg.is_absolute() {
            x.path_hg.clone()
        } else {
            config_path
                .map(|c| c.join(x.path_hg.clone()))
                .unwrap_or(x.path_hg.clone())
        };

        info!("Reading repo: {:?}", x.path_hg);
        let mercurial_repo = match MercurialRepo::open(&path, &x.config, env) {
            Ok(repo) => repo,
            Err(ErrorKind::HgParserFailure(fail)) => panic!("Cannot open {:?}: {:?}", path, fail),
            Err(other) => panic!("Cannot open {:?}: {:?}", path, other),
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

        let mut git_repo = GitTargetRepository::open(&x.path_git);

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

            for rev in min..max {
                debug!("Exporting commit: {}", rev);
                for mut changeset in mercurial_repo.range(min..max) {
                    c = mercurial_repo.export_commit(&mut changeset, max, c, &mut brmap, output)?;
                }
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
