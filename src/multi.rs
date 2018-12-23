use std::path::Path;
use std::path::PathBuf;

use log::{debug, info};

use cpython::{
    exc, GILGuard, NoArgs, ObjectProtocol, PyDict, PyErr, PyList, PyModule, PyObject, PyResult,
    PyString, PyStringData, Python,
};

use super::{config, MercurialToolkit, TargetRepository};

pub fn multi2git<P: AsRef<Path>>(
    _export_notes: bool,
    _verify: bool,
    _target: &mut TargetRepository,
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
    info!("Ok");
    Ok(())
}
