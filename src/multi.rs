use std::path::Path;
use std::path::PathBuf;

use cpython::{
    exc, GILGuard, NoArgs, ObjectProtocol, PyDict, PyErr, PyList, PyModule, PyObject, PyResult,
    PyString, PyStringData, Python,
};

use super::{config, MercurialToolkit, TargetRepository};

pub fn multi2git(
    _export_notes: bool,
    _verify: bool,
    _target: &mut TargetRepository,
    env: &config::Environment,
    multi_config: &config::MultiConfig,
) -> PyResult<()> {
    let gil = Python::acquire_gil();

    let mercurial = MercurialToolkit::new(&gil, env)?;

    let _repositories: Vec<_> = multi_config
        .repositories
        .iter()
        .map(|x| mercurial.open_repo(&x.path, &x.config).unwrap())
        .collect();
    Ok(())
}
