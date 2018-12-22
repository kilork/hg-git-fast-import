use std::path::Path;
use std::path::PathBuf;

use cpython::{
    exc, GILGuard, NoArgs, ObjectProtocol, PyDict, PyErr, PyList, PyModule, PyObject, PyResult,
    PyString, PyStringData, Python,
};

use super::{config, TargetRepository};

pub fn multi2git(
    _export_notes: bool,
    _verify: bool,
    _target: &mut TargetRepository,
    _env: &config::Environment,
    _config: &config::MultiConfig,
) -> PyResult<()> {
    Ok(())
}
