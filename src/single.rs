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

    let max = if let Some(limit_high) = repository_config.limit_high {
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
    let mut c: usize = 0;

    {
        let (output, saved_state) = target.init().unwrap();

        let min = if let Some(saved_state) = saved_state.as_ref() {
            match saved_state {
                RepositorySavedState::OffsetedRevision(rev) => {
                    rev - repo.config.offset.unwrap_or(0)
                }
            }
        } else {
            0
        };

        info!("Exporting commits from {}", min);

        for rev in min..max {
            debug!("exporting commit: {}", rev);
            for mut changeset in repo.range(min..max) {
                c = repo.export_commit(&mut changeset, max, c, &mut brmap, output)?;
            }
        }

        c = repo.export_tags(min..max, c, output)?;
    }
    info!("Issued {} commands", c);
    info!("Saving state...");
    target
        .save_state(RepositorySavedState::OffsetedRevision(
            max + repository_config.offset.unwrap_or(0),
        ))
        .unwrap();

    target.finish().unwrap();

    if verify {
        target
            .verify(
                repourl.as_ref().to_str().unwrap(),
                repository_config.path_prefix.as_ref().map(|x| &x[..]),
            )
            .unwrap();
    }

    Ok(())
}
