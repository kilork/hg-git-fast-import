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
    heads: HashMap<String, usize>,
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

            (
                x,
                match MercurialRepo::open(&path, &x.config, env) {
                    Ok(repo) => repo,
                    Err(ErrorKind::HgParserFailure(fail)) => {
                        panic!("Cannot open {:?}: {:?}", path, fail)
                    }
                    Err(other) => panic!("Cannot open {:?}: {:?}", path, other),
                },
            )
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
                        RepositorySavedState::OffsetedRevisionSet(saved_maxs)
                        | RepositorySavedState::HeadsAndOffsets {
                            offsets: saved_maxs,
                            ..
                        } => saved_maxs
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

                let heads = saved_state
                    .as_ref()
                    .and_then(|x| match x {
                        RepositorySavedState::OffsetedRevisionSet(_) => None,
                        RepositorySavedState::HeadsAndOffsets { repositories, .. } => repositories
                            .get(path_config.path.to_str().unwrap())
                            .map(|x| x.heads.clone()),
                    })
                    .unwrap_or_else(HashMap::new);

                let importing_repository = ImportingRepository {
                    min,
                    max,
                    brmap,
                    heads,
                    path_config,
                };
                info!(
                    "repo: {:?} min: {} max: {} offset: {:?}",
                    importing_repository.path_config.path,
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
        let mut all_repo_iterators_iter = conditional_multi_iter(
            all_repo_iterators,
            |x| x.iter().filter_map(|x| x.map(|x| x.0.header.time)).min(),
            |x, y| y == &x.map(|x| x.0.header.time),
        );

        let mut heads = saved_state
            .as_ref()
            .map(|x| match x {
                RepositorySavedState::HeadsAndOffsets { heads, .. } => heads.clone(),
                _ => HashMap::new(),
            })
            .unwrap_or_else(HashMap::new);

        info!("Exporting commits");

        for (ref mut changelog, repo_index) in &mut all_repo_iterators_iter {
            let importing_repository = &mut importing_repositories[repo_index];
            let (_, repo) = &repositories[repo_index];
            c = repo.export_commit(
                changelog,
                importing_repository.max,
                c,
                &mut importing_repository.brmap,
                &mut importing_repository.heads,
                &mut heads,
                output,
            )?;
        }

        for (importing_repo, (_, repo)) in importing_repositories.iter().zip(repositories.iter()) {
            c = repo.export_tags(importing_repo.min..importing_repo.max, c, output)?;
        }
        info!("Issued {} commands", c);
        info!("Saving state...");
        target
            .save_state(RepositorySavedState::HeadsAndOffsets {
                offsets: importing_repositories
                    .iter()
                    .map(|x| x.max + x.path_config.config.offset.unwrap_or(0))
                    .collect(),
                heads,
                repositories: importing_repositories
                    .iter()
                    .map(|x| {
                        (
                            x.path_config.path.to_str().unwrap().into(),
                            config::Repository {
                                heads: x.heads.clone(),
                            },
                        )
                    })
                    .collect(),
            })
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
