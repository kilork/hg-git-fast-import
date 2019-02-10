use std::collections::HashMap;
use std::path::Path;

use log::{debug, info};

use super::{
    collections::conditional_multi_iter, config, MercurialRepo, RepositorySavedState,
    TargetRepository,
};
use crate::error::ErrorKind;
use hg_parser::{Changeset, ChangesetIter};

struct ImportingRepository<'a> {
    min: usize,
    max: usize,
    brmap: HashMap<String, String>,
    path_config: &'a config::PathRepositoryConfig,
}

pub fn multi2git<P: AsRef<Path>>(
    verify: bool,
    target: &mut TargetRepository,
    env: &config::Environment,
    config: P,
    multi_config: &config::MultiConfig,
) -> Result<(), ErrorKind> {
    let config_path = config.as_ref().parent();

    let repositories: Vec<_> = multi_config
        .repositories
        .iter()
        .map(|x| {
            let path = if x.path.is_absolute() {
                x.path.clone()
            } else {
                config_path
                    .map(|c| c.join(x.path.clone()))
                    .unwrap_or(x.path.clone())
            };

            (x, MercurialRepo::open(&path, &x.config, env).unwrap())
        })
        .collect();

    if repositories.iter().any(|(path_config, repo)| {
        info!("Verifying heads in repository {:?}", path_config.path);
        !repo
            .verify_heads(path_config.config.allow_unnamed_heads)
            .unwrap()
    }) {
        return Err(ErrorKind::VerifyFailure("Verify heads failed".into()));
    }

    {
        let mut all_revisions = Vec::new();

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

                let brmap = path_config
                    .config
                    .branches
                    .as_ref()
                    .map(|x| x.clone())
                    .unwrap_or_else(|| HashMap::new());

                let importing_repository = ImportingRepository {
                    min: 0,
                    max,
                    brmap,
                    path_config,
                };

                all_revisions.push(ChangesetIterWrapper::new(
                    repo.range(importing_repository.min..importing_repository.max),
                    index,
                ));

                importing_repository
            })
            .collect();

        let mut c: usize = 0;
        let mut all_revisions_iter = conditional_multi_iter(
            all_revisions,
            |x| x.iter().filter_map(|x| x.map(|x| x.0.header.time)).min(),
            |x, y| y == &x.map(|x| x.0.header.time),
        );

        info!("Exporting commits");

        for (ref mut changelog, repo_index) in &mut all_revisions_iter {
            let importing_repository = &mut importing_repositories[repo_index];
            let (_, repo) = &repositories[repo_index];
            c = repo.export_commit(
                changelog,
                importing_repository.max,
                c,
                &mut importing_repository.brmap,
                output,
            )?;
        }

        for (importing_repo, (_, repo)) in importing_repositories.iter().zip(repositories.iter()) {
            c = repo.export_tags(importing_repo.min..importing_repo.max, c, output)?;
        }
        info!("Issued {} commands", c);
        info!("Saving state...");
        target
            .save_state(RepositorySavedState::OffsetedRevisionSet(
                importing_repositories
                    .iter()
                    .map(|x| x.max + x.path_config.config.offset.unwrap_or(0))
                    .collect(),
            ))
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

    Ok(())
}

struct ChangesetIterWrapper<'a> {
    repo_index: usize,
    changeset_iter: ChangesetIter<'a>,
}

impl<'a> ChangesetIterWrapper<'a> {
    fn new(changeset_iter: ChangesetIter<'a>, repo_index: usize) -> Self {
        Self {
            repo_index,
            changeset_iter,
        }
    }
}
impl<'a> Iterator for ChangesetIterWrapper<'a> {
    type Item = (Changeset, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.changeset_iter.next().map(|x| (x, self.repo_index))
    }
}
