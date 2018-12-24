use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use log::{debug, info};

use cpython::{
    exc, GILGuard, NoArgs, ObjectProtocol, PyDict, PyErr, PyList, PyModule, PyObject, PyResult,
    PyString, PyStringData, Python,
};

use super::{
    collections::conditional_multi_iter, config, MercurialRepo, MercurialToolkit, TargetRepository,
};

struct ImportingRepository<'a> {
    min: usize,
    max: usize,
    mapping_cache: HashMap<Vec<u8>, usize>,
    brmap: HashMap<String, String>,
    repo: MercurialRepo<'a>,
    path_config: &'a config::PathRepositoryConfig,
}

pub fn multi2git<P: AsRef<Path>>(
    _export_notes: bool,
    _verify: bool,
    target: &mut TargetRepository,
    env: &config::Environment,
    config: P,
    multi_config: &config::MultiConfig,
) -> PyResult<()> {
    let gil = Python::acquire_gil();

    let mercurial = MercurialToolkit::new(&gil, env)?;

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

            (x, mercurial.open_repo(&path, &x.config).unwrap())
        })
        .collect();

    if repositories.iter().any(|(path_config, repo)| {
        info!("Verifying heads in repository {:?}", path_config.path);
        !repo
            .verify_heads(path_config.config.allow_unnamed_heads)
            .unwrap()
    }) {
        return mercurial.error("Verify heads failed");
    }
<<<<<<< HEAD

    let mut all_revisions: Vec<Vec<(usize, usize, usize)>> = Vec::new();

    info!("Analyzing revision log");
    info!("Checking revision log to create single list of revisions in historical order");

    let mut repositories: Vec<_> = repositories
        .into_iter()
        .enumerate()
        .map(|(index, (path_config, repo))| {
            let tip = repo.changelog_len().unwrap();

            let max = if let Some(limit_high) = path_config.config.limit_high {
                tip.min(limit_high)
            } else {
                tip
            };

            let mut revisions = Vec::new();
            let mut mapping_cache = HashMap::new();
            for rev in 0..max {
                let (revnode, _, (time, _), _, _, _) = repo.changeset(rev).unwrap();
                mapping_cache.insert(revnode, rev);
                revisions.push((time, index, rev));
            }
            all_revisions.push(revisions);

            let brmap = path_config
                .config
                .branches
                .as_ref()
                .map(|x| x.clone())
                .unwrap_or_else(|| HashMap::new());

            let importing_repository = ImportingRepository {
                min: 0,
                max,
                mapping_cache,
                brmap,
                path_config,
                repo,
            };
            importing_repository
        })
        .collect();

    let mut c: usize = 0;
    let all_revisions_iter = conditional_multi_iter(
        &all_revisions,
        |x| x.iter().filter_map(|x| x.map(|x| x.0)).min(),
        |x, y| y == &x.map(|x| x.0),
    );
    {
        let (output, saved_state) = target.init().unwrap();

        info!("Exporting commits");

        for &(_, index, rev) in all_revisions_iter {
            let importing_repository = &mut repositories[index];
            c = importing_repository.repo.export_commit(
                rev,
                importing_repository.max,
                c,
                &mut importing_repository.brmap,
                output,
            )?;
        }
    }

    info!("Issued {} commands", c);

    target.finish().unwrap();

=======
    info!("Ok");
>>>>>>> c149ec30805c219b63fa9f157afb88746ae2279d
    Ok(())
}
