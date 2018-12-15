use lazy_static::lazy_static;

use log::{debug, error, info, warn};

use regex::Regex;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{
    self,
    prelude::{Read, Write},
};
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitStatus;

use cpython::{
    exc, GILGuard, NoArgs, ObjectProtocol, PyDict, PyErr, PyList, PyModule, PyObject, PyResult,
    PyString, PyStringData, Python,
};

pub mod config;
pub mod git;

use self::config::RepositorySavedState;

pub fn read_file(filename: &PathBuf) -> io::Result<String> {
    let mut file = File::open(filename)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

#[derive(Debug)]
pub enum TargetRepositoryError {
    Nope,
    IsNotDir,
    SavedStateDoesNotExist,
    CannotInitRepo(ExitStatus),
    CannotConfigRepo(ExitStatus),
    ImportFailed(ExitStatus),
    IOError(std::io::Error),
}

impl From<std::io::Error> for TargetRepositoryError {
    fn from(value: std::io::Error) -> Self {
        TargetRepositoryError::IOError(value)
    }
}

pub trait TargetRepository {
    fn init(
        &mut self,
    ) -> Result<(&mut Write, Option<config::RepositorySavedState>), TargetRepositoryError>;

    fn finish(&mut self, verify: Option<&str>) -> Result<(), TargetRepositoryError>;

    fn save_state(&self, _state: RepositorySavedState) -> std::io::Result<()> {
        Ok(())
    }

    fn get_saved_state(&self) -> Option<&RepositorySavedState> {
        None
    }
}

struct MercurialToolkit<'a> {
    py: Python<'a>,
    _mercurial: PyModule,
    mercurial_hg: PyModule,
    mercurial_node: PyModule,
    mercurial_scmutil: PyModule,
    ui: PyObject,
    env: &'a config::Environment,
}

type Changeset = (Vec<u8>, String, (usize, String), String, String, bool);

struct MercurialRepo<'a> {
    toolkit: &'a MercurialToolkit<'a>,
    repo: PyObject,
    changelog: PyObject,
    config: &'a config::RepositoryConfig,
    changeset_cache: RefCell<HashMap<usize, Changeset>>,
}

impl<'a> MercurialToolkit<'a> {
    fn new(gil: &'a GILGuard, env: &'a config::Environment) -> PyResult<Self> {
        let py = gil.python();
        let _mercurial = py.import("mercurial")?;
        let mercurial_ui = py.import("mercurial.ui")?;
        let mercurial_hg = py.import("mercurial.hg")?;
        let mercurial_node = py.import("mercurial.node")?;
        let mercurial_scmutil = py.import("mercurial.scmutil")?;
        let ui = mercurial_ui.call(py, "ui", NoArgs, None)?;
        ui.call_method(py, "setconfig", ("ui", "interactive", false), None)?;
        Ok(Self {
            py,
            _mercurial,
            mercurial_hg,
            mercurial_node,
            mercurial_scmutil,
            ui,
            env,
        })
    }

    fn open_repo<P: AsRef<Path>>(
        &self,
        url: P,
        config: &'a config::RepositoryConfig,
    ) -> PyResult<MercurialRepo> {
        let py = self.py;
        let repo =
            self.mercurial_hg
                .call(py, "repository", (&self.ui, url.as_ref().to_str()), None)?;

        let changelog = repo.getattr(py, "changelog")?;
        Ok(MercurialRepo {
            toolkit: self,
            repo,
            changelog,
            config,
            changeset_cache: RefCell::new(HashMap::new()),
        })
    }

    fn error<T>(&self, error_text: &str) -> PyResult<T> {
        Err(PyErr::new::<exc::Exception, _>(self.py, error_text))
    }
}

impl<'a> MercurialRepo<'a> {
    fn verify_heads(&self, allow_unnamed_heads: bool) -> PyResult<bool> {
        let (py, repo, changelog) = (self.toolkit.py, &self.repo, &self.changelog);
        let branchmap: PyDict = repo
            .call_method(py, "branchmap", NoArgs, None)?
            .extract(py)?;

        let branchmap: HashMap<String, PyList> = branchmap
            .items(py)
            .iter()
            .map(|(k, v)| (k.extract(py).unwrap(), v.extract(py).unwrap()))
            .collect();

        let mut branches = HashMap::new();
        for (bn, heads) in &branchmap {
            let tip = branchtip(py, changelog, heads)?;
            branches.insert(bn, tip);
        }
        debug!("branches: {:?}", branches.keys());

        debug!("branchmap len: {:?}", branchmap.len());

        // TODO: all code above is doing nothing if we do not have cache, should be removed
        info!("Verify that branch has exactly one head");
        let heads: PyList = repo.call_method(py, "heads", NoArgs, None)?.extract(py)?;
        let mut t = HashSet::new();
        for h in heads.iter(py) {
            let rev: usize = changelog.call_method(py, "rev", (h,), None)?.extract(py)?;
            let (_, _, _, _, branch, _) = self.changeset(rev)?;
            if t.contains(&branch) {
                if allow_unnamed_heads {
                    warn!(
                        "repository has at least one unnamed head: hg r{}, branch: {}",
                        rev, branch
                    );
                } else {
                    error!(
                        "repository has at least one unnamed head: hg r{}, branch: {}",
                        rev, branch
                    );
                    return Ok(false);
                }
            }
            t.insert(branch);
        }
        Ok(true)
    }

    fn changelog_len(&self) -> PyResult<usize> {
        self.changelog.len(self.toolkit.py)
    }

    fn fixup_user(&self, user: &str) -> String {
        if let Some(ref authors) = self.config.authors {
            if let Some(remap) = authors.get(user) {
                return remap.clone();
            }
        }

        if let Some(ref authors) = self.toolkit.env.authors {
            if let Some(remap) = authors.get(user) {
                return remap.clone();
            }
        }

        lazy_static! {
            static ref RE: Regex = Regex::new("([^<]+) ?(<[^>]*>)$").unwrap();
        }

        let (name, email) = if let Some(caps) = RE.captures(&user) {
            (
                caps.get(1).unwrap().as_str().trim_right(),
                caps.get(2).unwrap().as_str(),
            )
        } else {
            (&user[..], "<unknown@localhost>")
            // panic!("Wrong user: {}", user);
        };

        format!("{} {}", name, email)
    }

    fn revsymbol(&self, revision: usize) -> PyResult<PyObject> {
        let (py, repo, scmutil) = (self.toolkit.py, &self.repo, &self.toolkit.mercurial_scmutil);
        let revsymbol = scmutil.call(py, "revsymbol", (repo, revision.to_string()), None)?;
        Ok(revsymbol)
    }

    fn changeset(&self, revision: usize) -> PyResult<Changeset> {
        let (py, changelog, scmutil) = (
            self.toolkit.py,
            &self.changelog,
            &self.toolkit.mercurial_scmutil,
        );

        debug!("get changeset for revision: {:?}", revision);

        if let Some(result) = self.changeset_cache.borrow().get(&revision) {
            return Ok(result.clone());
        }

        let node: PyString = {
            let revsymbol = self.revsymbol(revision)?;
            scmutil
                .call(py, "binnode", (revsymbol,), None)?
                .extract(py)?
        };

        let revision_read = changelog.call_method(py, "read", (revision,), None)?;
        let time_data = revision_read.get_item(py, 2)?;
        let (user, time, timezone, desc, extra) = (
            revision_read.get_item(py, 1)?,
            time_data.get_item(py, 0)?,
            time_data.get_item(py, 1)?,
            revision_read.get_item(py, 4)?,
            revision_read.get_item(py, 5)?,
        );
        let time: usize = time.extract(py)?;
        let timezone: i32 = timezone.extract(py)?;
        let tz = format!("{:+03}{:02}", -timezone / 3600, ((-timezone % 3600) / 60));
        let extra: PyDict = extra.extract(py)?;
        let branch = get_branch(extra.get_item(py, "branch").map(|p| p.extract(py).unwrap()));
        let is_closed = extra.contains(py, "close")?;

        let (_, data) = convert_pystring_to_bytes(py, &node);
        let user: String = user.extract(py)?;
        let result = (
            Vec::from(data),
            self.fixup_user(&user),
            (time, tz),
            desc.extract(py)?,
            branch,
            is_closed,
        );

        let mut changeset_cache = self.changeset_cache.borrow_mut();
        changeset_cache.insert(revision, result.clone());

        Ok(result)
    }

    fn manifest(&self, ctx: &PyObject, _revision: usize) -> PyResult<PyObject> {
        let manifest = ctx.call_method(self.toolkit.py, "manifest", NoArgs, None)?;
        Ok(manifest)
    }

    fn export_commit(
        &self,
        revision: usize,
        max: usize,
        count: usize,
        brmap: &mut HashMap<String, String>,
        output: &mut Write,
    ) -> PyResult<usize> {
        let (py, env, repo) = (self.toolkit.py, self.toolkit.env, &self.repo);
        let (_, user, (time, timezone), desc, branch, is_closed) = self.changeset(revision)?;

        let branch = brmap
            .entry(branch.clone())
            .or_insert_with(|| sanitize_name(&branch, "branch"));

        let parents = self.get_parents(revision)?;

        debug!("parents: {:?}", parents);

        if !parents.is_empty() && revision != 0 {
            writeln!(output, "reset refs/heads/{}", branch).unwrap();
        }
        writeln!(output, "commit refs/heads/{}", branch).unwrap();
        writeln!(output, "mark :{}", revision + 1).unwrap();
        writeln!(
            output,
            "author {} {} {}",
            get_author(&desc, &user),
            time,
            timezone
        )
        .unwrap();
        writeln!(output, "committer {} {} {}", user, time, timezone).unwrap();
        writeln!(output, "data {}", desc.len() + 1).unwrap();
        writeln!(output, "{}\n", desc).unwrap();

        let ctx = self.revsymbol(revision)?;
        let man = self.manifest(&ctx, revision)?;

        let mut added = vec![];
        let mut changed = vec![];
        let mut removed = vec![];

        let rev_type = if parents.is_empty() {
            let mut man_keys = get_keys(py, &man)?;
            added.append(&mut man_keys);
            added.sort();
            "full"
        } else {
            let parent = parents[0];
            writeln!(output, "from :{}", parent + 1).unwrap();
            if parents.len() == 1 {
                let mut f: Vec<Vec<String>> = repo
                    .call_method(py, "status", (parent, revision), None)?
                    .iter(py)?
                    .take(3)
                    .map(|x| x.unwrap().extract(py).unwrap())
                    .collect();
                added.append(&mut f[1]);
                changed.append(&mut f[0]);
                removed.append(&mut f[2]);
                "simple delta"
            } else {
                writeln!(output, "merge :{}", parents[1] + 1).unwrap();
                let (mut a, mut c, mut r) = self.get_filechanges(&parents, &man)?;
                added.append(&mut a);
                changed.append(&mut c);
                removed.append(&mut r);
                "thorough delta"
            }
        };
        info!(
            "{}: Exporting {} revision {}/{} with {}/{}/{} added/changed/removed files",
            branch,
            rev_type,
            revision + 1,
            max,
            added.len(),
            changed.len(),
            removed.len()
        );

        removed
            .iter()
            .map(strip_leading_slash)
            .for_each(|x| writeln!(output, "D {}", x).unwrap());
        export_file_contents(py, &ctx, &man, &added, output)?;
        export_file_contents(py, &ctx, &man, &changed, output)?;
        writeln!(output).unwrap();
        if is_closed && !env.no_clean_closed_branches {
            info!(
                "Saving reference to closed branch {} as archive/{}",
                branch, branch
            );
            writeln!(output, "reset refs/tags/archive/{}", branch).unwrap();
            writeln!(output, "from :{}\n", revision + 1).unwrap();

            info!("Closing branch: {}", branch);
            writeln!(output, "reset refs/heads/{}", branch).unwrap();
            writeln!(output, "from 0000000000000000000000000000000000000000\n").unwrap();
        }
        Ok(count + 1)
    }

    fn export_note(
        &self,
        revision: usize,
        count: usize,
        is_first: bool,
        output: &mut Write,
    ) -> PyResult<usize> {
        let py = self.toolkit.py;
        let (_, user, (time, timezone), _, _, _) = self.changeset(revision)?;

        writeln!(output, "commit refs/notes/hg").unwrap();
        writeln!(output, "committer {} {} {}", user, time, timezone).unwrap();
        writeln!(output, "data 0").unwrap();
        if is_first {
            writeln!(output, "from refs/notes/hg^0").unwrap();
        }
        writeln!(output, "N inline :{}", revision + 1).unwrap();

        let ctx = self.revsymbol(revision)?;
        let hg_hash: String = ctx.call_method(py, "hex", NoArgs, None)?.extract(py)?;

        writeln!(output, "data {}", hg_hash.len()).unwrap();
        writeln!(output, "{}", hg_hash).unwrap();

        Ok(count + 1)
    }

    fn export_tags(
        &self,
        mapping_cache: &HashMap<Vec<u8>, usize>,
        mut count: usize,
        output: &mut Write,
    ) -> PyResult<usize> {
        let (py, repo) = (self.toolkit.py, &self.repo);
        let l: Vec<(String, PyObject)> = repo
            .call_method(py, "tagslist", NoArgs, None)?
            .extract(py)?;

        for (tag, node) in l {
            let tag = sanitize_name(&tag, "tag");
            if tag == "tip" {
                continue;
            }

            let node_str: PyString = node.extract(py)?;
            let (_, node_key) = convert_pystring_to_bytes(py, &node_str);
            if let Some(rev) = mapping_cache.get(node_key) {
                writeln!(output, "reset refs/tags/{}", tag).unwrap();
                writeln!(output, "from :{}", rev + 1).unwrap();
                writeln!(output).unwrap();
                count += 1;
            } else {
                error!("Tag {} refers to unseen node {:?}", tag, node_key);
            }
        }
        Ok(count)
    }

    fn get_parents(&self, revision: usize) -> PyResult<Vec<i32>> {
        let py = self.toolkit.py;
        Ok(self
            .changelog
            .call_method(py, "parentrevs", (revision,), None)?
            .extract::<Vec<i32>>(py)?
            .into_iter()
            .filter(|&p| p >= 0)
            .collect())
    }

    fn get_filechanges(
        &self,
        parents: &[i32],
        mleft: &PyObject,
    ) -> PyResult<(Vec<String>, Vec<String>, Vec<String>)> {
        let (py, node) = (self.toolkit.py, &self.toolkit.mercurial_node);
        let (mut l, mut c, mut r) = (vec![], vec![], vec![]);
        for &p in parents {
            if p < 0 {
                continue;
            }
            let rev = p as usize;
            let ctx = self.revsymbol(rev)?;
            let mright = self.manifest(&ctx, rev)?;
            split_dict(py, node, mleft, &mright, &mut l, &mut c, &mut r)?;
        }
        l.sort();
        c.sort();
        r.sort();
        Ok((l, c, r))
    }
}

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
    let mut brmap = HashMap::new();
    let mut c: usize = 0;

    {
        let (output, saved_state) = target.init().unwrap();

        let min = if let Some(saved_state) = saved_state {
            match saved_state {
                RepositorySavedState::OffsetedRevisionSet(revs) => *revs.first().unwrap(),
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
        .save_state(RepositorySavedState::OffsetedRevisionSet(vec![max]))
        .unwrap();

    target
        .finish(if verify {
            Some(repourl.as_ref().to_str().unwrap())
        } else {
            None
        })
        .unwrap();
    Ok(())
}

fn convert_pystring_to_bytes<'a>(py: Python, py_str: &'a PyString) -> (usize, &'a [u8]) {
    let data = py_str.data(py);
    match data {
        PyStringData::Utf8(bytes) => (bytes.len(), bytes),
        _ => panic!(),
    }
}

fn export_file_contents(
    py: Python,
    ctx: &PyObject,
    manifest: &PyObject,
    files: &[String],
    output: &mut Write,
) -> PyResult<()> {
    for file in files {
        if file == ".hgtags" {
            info!("Skip {}", file);
            continue;
        }
        let file_ctx = ctx.call_method(py, "filectx", (file,), None)?;
        let data = file_ctx.call_method(py, "data", NoArgs, None)?;
        let data_str: PyString = data.extract(py)?;
        writeln!(
            output,
            "M {} inline {}",
            gitmode(&get_flags(py, manifest, file)?),
            strip_leading_slash(file)
        )
        .unwrap();
        let (data_str_len, data_str_bytes) = convert_pystring_to_bytes(py, &data_str);
        writeln!(output, "data {}", data_str_len).unwrap();
        output.write_all(data_str_bytes).unwrap();
        writeln!(output).unwrap();
    }
    Ok(())
}

fn strip_leading_slash(x: &String) -> String {
    x.to_string()
}

fn get_keys(py: Python, d: &PyObject) -> PyResult<Vec<String>> {
    d.call_method(py, "keys", NoArgs, None)?.extract(py)
}

fn get_flags(py: Python, d: &PyObject, item: &str) -> PyResult<String> {
    d.call_method(py, "flags", (item,), None)?.extract(py)
}

fn split_dict(
    py: Python,
    node: &PyModule,
    dleft: &PyObject,
    dright: &PyObject,
    l: &mut Vec<String>,
    c: &mut Vec<String>,
    r: &mut Vec<String>,
) -> PyResult<()> {
    for left in &get_keys(py, dleft)? {
        let right = dright.get_item(py, left).ok();
        if right.is_none() {
            l.push(left.to_string());
        } else if file_mismatch(py, node, dleft.get_item(py, left).ok(), right)?
            || gitmode(&get_flags(py, dleft, left)?) != gitmode(&get_flags(py, dright, left)?)
        {
            c.push(left.to_string());
        }
    }
    for right in &get_keys(py, dright)? {
        let left = dleft.get_item(py, right).ok();
        if left.is_none() {
            r.push(right.to_string());
        }
    }
    Ok(())
}

fn gitmode(flags: &str) -> &'static str {
    if flags.contains('l') {
        return "120000";
    }
    if flags.contains('x') {
        return "100755";
    }
    "100644"
}

fn hex(py: Python, node: &PyModule, filenode: &PyObject) -> String {
    node.call(py, "hex", (filenode,), None)
        .unwrap()
        .extract::<String>(py)
        .unwrap()
}

fn file_mismatch(
    py: Python,
    node: &PyModule,
    f1: Option<PyObject>,
    f2: Option<PyObject>,
) -> PyResult<bool> {
    let f1 = f1.map(|ref x| hex(py, node, x));
    let f2 = f2.map(|ref x| hex(py, node, x));
    Ok(f1 != f2)
}

fn get_author(logmessage: &str, committer: &str) -> String {
    if logmessage.contains("Signed") {
        warn!("Probably need to implement this, because logmessage contains different author");
        warn!("logmessage: {}", logmessage);
        warn!("committer: {}", committer);
    }
    committer.into()
}

fn sanitize_name(name: &str, _what: &str) -> String {
    name.into()

    //TODO: git-check-ref-format
}

fn branchtip(py: Python, changelog: &PyObject, heads: &PyList) -> PyResult<PyString> {
    let tip = heads.get_item(py, heads.len(py) - 1).extract(py)?;
    let heads: Vec<_> = heads.iter(py).collect();
    for h in heads.iter().rev() {
        let is_closed = changelog
            .call_method(py, "read", (h,), None)?
            .get_item(py, 5)?
            .extract::<PyDict>(py)?
            .contains(py, "close")?;
        if !is_closed {
            return Ok(h.extract(py)?);
        }
    }
    Ok(tip)
}

fn get_branch(name: Option<String>) -> String {
    match name.as_ref().map(|x| &x[..]) {
        Some("HEAD") | Some("default") | Some("") | None => "master".into(),
        Some(name) => name.into(),
    }
}
