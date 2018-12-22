use std::path::Path;
use std::path::PathBuf;

use cpython::{
    exc, GILGuard, NoArgs, ObjectProtocol, PyDict, PyErr, PyList, PyModule, PyObject, PyResult,
    PyString, PyStringData, Python,
};

use super::{config, TargetRepository};

pub fn multi2git(
    export_notes: bool,
    verify: bool,
    target: &mut TargetRepository,
    env: &config::Environment,
    config: &config::MultiConfig,
) -> PyResult<()> {
    Ok(())
}
