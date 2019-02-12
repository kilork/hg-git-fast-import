use std::collections::HashMap;
use std::path::Path;

use log::{debug, info};

use super::{
    config, MercurialRepo, RepositorySavedState,
    TargetRepository,
};
use crate::error::ErrorKind;
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

    multi_config
        .repositories
        .iter()
        .for_each(|x| {
            let path = if x.path_hg.is_absolute() {
                x.path_hg.clone()
            } else {
                config_path
                    .map(|c| c.join(x.path_hg.clone()))
                    .unwrap_or(x.path_hg.clone())
            };

                let mercurial_repo = match MercurialRepo::open(&path, &x.config, env) {
                    Ok(repo) => repo,
                    Err(ErrorKind::HgParserFailure(fail)) => {
                        panic!("Cannot open {:?}: {:?}", path, fail)
                    }
                    Err(other) => panic!("Cannot open {:?}: {:?}", path, other),
                };
        });
/*
    if repositories.iter().any(|(path_config, repo)| {
        info!("Verifying heads in repository {:?}", path_config.path_hg);
        !repo
            .verify_heads(path_config.config.allow_unnamed_heads)
            .unwrap()
    }) {
        return Err(ErrorKind::VerifyFailure("Verify heads failed".into()));
    }

    {
        let mut all_repo_iterators = Vec::new();

        info!("Trying to load state");
        let (output, saved_state) = target.init().unwrap();

        info!("Checking revision log to create single list of revisions in historical order");

        let mut importing_repositories: Vec<_> = repositories
            .iter()
            .enumerate()
            .map(|(index, (path_config, repo))| {
                let tip = repo.changelog_len().unwrap();

                let max = if let Some(limit_high) = path_config.config.limit_high {
                    tip.min(limit_high)
                } else {
                    tip
                };

                let offset = path_config.config.offset.unwrap_or(0);

                let min = saved_state
                    .as_ref()
                    .and_then(|x| match x {
                        RepositorySavedState::OffsetedRevisionSet(saved_maxs) => saved_maxs
                            .iter()
                            .filter(|&&rev| rev >= offset && rev <= max + offset)
                            .map(|x| x - offset)
                            .next(),
                    })
                    .unwrap_or(0);

                let brmap = path_config
                    .config
                    .branches
                    .as_ref()
                    .map(|x| x.clone())
                    .unwrap_or_else(|| HashMap::new());

                let importing_repository = ImportingRepository {
                    min,
                    max,
                    brmap,
                    path_config,
                };
                info!(
                    "repo: {:?} min: {} max: {} offset: {:?}",
                    importing_repository.path_config.path_hg,
                    importing_repository.min,
                    importing_repository.max,
                    importing_repository.path_config.config.offset
                );

                all_repo_iterators.push(ChangesetIterWrapper::new(
                    repo.range(importing_repository.min..importing_repository.max),
                    index,
                ));

                importing_repository
            })
            .collect();

        let mut c: usize = 0;

        info!("Exporting commits");

        info!("Issued {} commands", c);
        info!("Saving state...");
        target
            .save_state(RepositorySavedState::OffsetedRevision(importing_repository.max))
            .unwrap();
    }

    target.finish().unwrap();

    if verify {
        for (repository_config, repo) in repositories {
            target
                .verify(
                    repo.path().to_str().unwrap(),
                    repository_config
                        .config
                        .path_prefix
                        .as_ref()
                        .map(|x| &x[..]),
                )
                .unwrap();
        }
    }
*/
    Ok(())
}
