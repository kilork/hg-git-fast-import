use std::collections::HashMap;

use log::{debug, info};
use std::path::Path;

use crate::error::ErrorKind;

use super::{config, MercurialRepo, RepositorySavedState, TargetRepository};

pub fn hg2git<P: AsRef<Path>>(
    repourl: P,
    verify: bool,
    target: &mut TargetRepository,
    env: &config::Environment,
    repository_config: &config::RepositoryConfig,
) -> Result<(), ErrorKind> {
    let repo = MercurialRepo::open(repourl.as_ref(), repository_config, env)?;

    if !repo.verify_heads(repository_config.allow_unnamed_heads)? {
        return Err(ErrorKind::VerifyFailure("Verify heads failed".into()));
    };

    let tip = repo.changelog_len()?;

    let to = if let Some(limit_high) = repository_config.limit_high {
        tip.min(limit_high)
    } else {
        tip
    };

    debug!("Checking saved state...");
    let mut brmap = repository_config
        .branches
        .as_ref()
        .map(|x| x.clone())
        .unwrap_or_else(|| HashMap::new());
    let mut counter: usize = 0;

    {
        let (output, saved_state) = target.start_import()?;

        let from = if let Some(saved_state) = saved_state.as_ref() {
            match saved_state {
                RepositorySavedState::OffsetedRevision(rev) => {
                    rev - repo.config.offset.unwrap_or(0)
                }
            }
        } else {
            0
        };

        info!("Exporting commits from {}", from);

        for mut changeset in repo.range(from..to) {
            counter = repo.export_commit(&mut changeset, counter, &mut brmap, output)?;
        }

        counter = repo.export_tags(from..to, counter, output)?;
    }
    info!("Issued {} commands", counter);
    info!("Saving state...");
    target.save_state(RepositorySavedState::OffsetedRevision(
        to + repository_config.offset.unwrap_or(0),
    ))?;

    target.finish()?;

    if verify {
        target.verify(
            repourl.as_ref().to_str().unwrap(),
            repository_config.path_prefix.as_ref().map(|x| &x[..]),
        )?;
    }

    Ok(())
}
