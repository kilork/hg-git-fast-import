use std::collections::{HashMap, HashSet};

use log::{debug, error, info, trace, warn};
use std::path::Path;
use std::path::PathBuf;

use cpython::{
    exc, GILGuard, NoArgs, ObjectProtocol, PyDict, PyErr, PyList, PyModule, PyObject, PyResult,
    PyString, PyStringData, Python,
};

use super::{config, MercurialToolkit, RepositorySavedState, TargetRepository};

pub fn hg2git<P: AsRef<Path>>(
    repourl: P,
    export_notes: bool,
    verify: bool,
    target: &mut TargetRepository,
    env: &config::Environment,
    config: &config::RepositoryConfig,
) -> PyResult<()> {
    let gil = Python::acquire_gil();

    let mercurial = MercurialToolkit::new(&gil, env)?;

    let repo = mercurial.open_repo(&repourl, config)?;

    if !repo.verify_heads(config.allow_unnamed_heads)? {
        return mercurial.error("Verify heads failed");
    };

    let tip = repo.changelog_len()?;

    let max = if let Some(limit_high) = config.limit_high {
        tip.min(limit_high)
    } else {
        tip
    };

    let mut mapping_cache = HashMap::new();
    for rev in 0..max {
        let (revnode, _, _, _, _, _) = repo.changeset(rev)?;
        mapping_cache.insert(revnode, rev);
    }
    debug!("mapping_cache: {:?}", mapping_cache);

    debug!("Checking saved state...");
    let mut brmap = config
        .branches
        .as_ref()
        .map(|x| x.clone())
        .unwrap_or_else(|| HashMap::new());
    let mut c: usize = 0;

    {
        let (output, saved_state) = target.init().unwrap();

        let min = if let Some(saved_state) = saved_state {
            match saved_state {
                RepositorySavedState::OffsetedRevisionSet(revs) => {
                    *revs.first().unwrap() - repo.config.offset.unwrap_or(0)
                }
            }
        } else {
            0
        };

        info!("Exporting commits from {}", min);

        for rev in min..max {
            debug!("exporting commit: {}", rev);
            c = repo.export_commit(rev, max, c, &mut brmap, output)?;
        }

        if export_notes {
            for rev in min..max {
                c = repo.export_note(rev, c, rev == min && min != 0, output)?;
            }
        }

        c = repo.export_tags(&mapping_cache, c, output)?;
    }
    info!("Issued {} commands", c);
    info!("Saving state...");
    target
        .save_state(RepositorySavedState::OffsetedRevisionSet(vec![
            max + config.offset.unwrap_or(0),
        ]))
        .unwrap();

    target.finish().unwrap();

    if verify {
        target
            .verify(
                repourl.as_ref().to_str().unwrap(),
                config.path_prefix.as_ref().map(|x| &x[..]),
            )
            .unwrap();
    }

    Ok(())
}
