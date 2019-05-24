use super::read_file;
use super::{
    config::RepositorySavedState, env::Environment, TargetRepository, TargetRepositoryError,
};

use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use log::{debug, error, info};
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::process::{Child, Command, ExitStatus, Stdio};

pub struct StdoutTargetRepository<'a> {
    stdoutlock: std::io::StdoutLock<'a>,
}

impl<'a> From<std::io::StdoutLock<'a>> for StdoutTargetRepository<'a> {
    fn from(value: std::io::StdoutLock<'a>) -> Self {
        Self { stdoutlock: value }
    }
}

impl<'a> TargetRepository for StdoutTargetRepository<'a> {
    fn start_import(
        &mut self,
        _git_active_branches: Option<usize>,
    ) -> Result<(&mut Write, Option<RepositorySavedState>), TargetRepositoryError> {
        Ok((&mut self.stdoutlock, None))
    }
    fn finish(&mut self) -> Result<(), TargetRepositoryError> {
        Ok(())
    }
}

pub struct GitTargetRepository<'a> {
    path: PathBuf,
    fast_import_cmd: Option<Child>,
    saved_state: Option<RepositorySavedState>,
    env: Option<&'a Environment>,
}

impl<'a> GitTargetRepository<'a> {
    pub fn open<P: AsRef<Path>>(value: P) -> Self {
        Self {
            path: value.as_ref().into(),
            fast_import_cmd: None,
            saved_state: None,
            env: None,
        }
    }

    pub fn set_env(&mut self, value: &'a Environment) {
        self.env = Some(value);
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn get_saved_state_path(&self) -> PathBuf {
        let mut saved_state = self.path.join(".git").join(env!("CARGO_PKG_NAME"));
        saved_state.set_extension("lock");
        saved_state
    }

    pub fn create_repo(&self) -> Result<(), TargetRepositoryError> {
        let path = &self.path;
        info!("Creating new dir");
        fs::create_dir_all(path)?;

        info!("Init Git repo");
        let status = Command::new("git").arg("init").current_dir(path).status()?;
        if !status.success() {
            error!("Cannot init Git repo");
            return Err(TargetRepositoryError::CannotInitRepo(status));
        }

        info!("Configure Git repo");
        self.git_config("core.ignoreCase", "false")?;

        info!("New Git repo initialization done");

        Ok(())
    }

    pub fn git_cmd(&self, args: &[&str]) -> Command {
        let mut git_cmd = Command::new("git");
        git_cmd.current_dir(&self.path).args(args);
        git_cmd
    }

    fn git_config(&self, key: &str, value: &str) -> Result<(), TargetRepositoryError> {
        let status = self.git_cmd(&["config", key, value]).status()?;

        if !status.success() {
            error!("Cannot configure Git repo");
            return Err(TargetRepositoryError::CannotConfigRepo(status));
        }

        Ok(())
    }

    fn git(&self, args: &[&str], quiet: bool) -> ExitStatus {
        self.git_cmd_quiet(|cmd| cmd.args(args), quiet)
    }

    fn git_cmd_quiet<F>(&self, mut f: F, quiet: bool) -> ExitStatus
    where
        F: FnMut(&mut Command) -> &mut Command,
    {
        self.git_cmd_status(|mut cmd| {
            f(&mut cmd);
            if quiet {
                cmd.arg("--quiet");
            }
        })
    }

    fn git_cmd_status<F>(&self, mut f: F) -> ExitStatus
    where
        F: FnMut(&mut Command),
    {
        let mut git_cmd = Command::new("git");
        f(&mut git_cmd);
        git_cmd.current_dir(&self.path).status().unwrap()
    }
}

impl<'a> TargetRepository for GitTargetRepository<'a> {
    fn start_import(
        &mut self,
        git_active_branches: Option<usize>,
    ) -> Result<(&mut Write, Option<RepositorySavedState>), TargetRepositoryError> {
        let path = &self.path;
        let saved_state;
        info!("Checking Git repo: {}", path.to_str().unwrap());

        let clean = self.env.map(|x| x.clean).unwrap_or_default();
        if path.exists() && clean {
            info!("Path exists, removing because of clean option");
            std::fs::remove_dir_all(path)?;
        }

        if path.exists() {
            if path.is_dir() {
                info!("Path exists, checking for saved state");

                let saved_state_path = self.get_saved_state_path();

                if !saved_state_path.exists() {
                    return Err(TargetRepositoryError::SavedStateDoesNotExist);
                }

                let saved_state_str = read_file(&saved_state_path)?;
                let loaded_saved_state: RepositorySavedState =
                    toml::from_str(&saved_state_str).unwrap();

                info!("Loaded saved state: {:?}", loaded_saved_state);
                saved_state = Some(loaded_saved_state);
            } else {
                error!("Path must be directory");
                return Err(TargetRepositoryError::IsNotDir);
            }
        } else {
            self.create_repo()?;
            saved_state = None;
        }

        let mut git = Command::new("git");
        let mut git_cmd = git.args(&[
            "fast-import",
            "--export-marks=.git/hg-git-fast-import.marks",
            "--import-marks-if-exists=.git/hg-git-fast-import.marks",
            "--quiet",
        ]);
        if let Some(git_active_branches) = git_active_branches {
            git_cmd = git_cmd.arg(format!("--active-branches={}", git_active_branches));
        }
        self.fast_import_cmd = Some(git_cmd.current_dir(path).stdin(Stdio::piped()).spawn()?);

        Ok((
            self.fast_import_cmd
                .as_mut()
                .map(|x| x.stdin.as_mut().unwrap())
                .unwrap(),
            saved_state,
        ))
    }

    fn finish(&mut self) -> Result<(), TargetRepositoryError> {
        info!("Waiting for Git fast-import to finish");

        let status = self.fast_import_cmd.as_mut().unwrap().wait()?;
        info!("Finished");

        let cron = self.env.map(|x| x.cron).unwrap_or_default();

        let status = if status.success() {
            info!("Checking out HEAD revision");
            self.git(&["checkout", "HEAD"], cron)
        } else {
            error!("Git fast-import failed.");
            return Err(TargetRepositoryError::ImportFailed(status));
        };

        let status = if status.success() {
            info!("Resetting Git repo.");
            self.git(&["reset", "--hard"], cron)
        } else {
            panic!("Cannot checkout HEAD revision in Git repo.")
        };

        let status = if status.success() {
            info!("Cleanup Git repo");
            self.git(&["clean", "-d", "-x", "-f"], cron)
        } else {
            panic!("Cannot reset Git repo.")
        };
        if !status.success() {
            panic!("Cannot cleanup Git repo.");
        };

        let (target_push, target_pull) = self
            .env
            .map(|x| (x.target_push, x.target_pull))
            .unwrap_or_default();

        if target_pull {
            info!("Pulling Git repo.");
            self.fetch_all()?;
        }

        if target_push {
            info!("Pushing Git repo.");
            let status = self.git(&["push", "--all"], cron);
            let status = if status.success() {
                self.git(&["push", "--tags"], cron)
            } else {
                panic!("Cannot push all to Git repo.");
            };
            if !status.success() {
                panic!("Cannot push all tags to Git repo.");
            };
        }

        Ok(())
    }

    fn verify(
        &self,
        verified_repo: &str,
        subfolder: Option<&str>,
    ) -> Result<(), TargetRepositoryError> {
        info!("Verifying...");

        let path: String = subfolder.map_or_else(
            || self.path.to_str().unwrap().into(),
            |subfolder| self.path.join(subfolder).to_str().unwrap().into(),
        );

        info!(
            "Verify - Mercurial (source): {} vs Git (target): {}",
            verified_repo, path
        );
        let status = Command::new("diff")
            .args(&[
                "-ur",
                "--exclude=.hg",
                "--exclude=.idea",
                "--exclude=.git",
                "--exclude=*.iml",
                "--exclude=target",
                "--exclude=.hgtags",
                verified_repo,
                &path,
            ])
            .status()
            .unwrap();
        if status.success() {
            Ok(())
        } else {
            Err(TargetRepositoryError::VerifyFail)
        }
    }

    fn get_saved_state(&self) -> Option<&RepositorySavedState> {
        self.saved_state.as_ref()
    }

    fn save_state(&self, state: RepositorySavedState) -> Result<(), TargetRepositoryError> {
        let path = &self.path;
        info!("Saving state to Git repo: {}", path.to_str().unwrap());
        let saved_state_path = self.get_saved_state_path();
        let toml = toml::to_string(&state).unwrap();
        let mut f = File::create(&saved_state_path)?;
        f.write_all(toml.as_bytes())?;
        Ok(())
    }

    fn remote_list(&self) -> Result<HashSet<String>, TargetRepositoryError> {
        debug!("git remote");
        let output = Command::new("git")
            .arg("remote")
            .current_dir(&self.path)
            .output()?;
        Ok(output
            .stdout
            .split(|&x| x == b'\n')
            .filter_map(|x| {
                if !x.is_empty() {
                    Some(std::str::from_utf8(x).unwrap().into())
                } else {
                    None
                }
            })
            .collect())
    }

    fn remote_add(&self, name: &str, url: &str) -> Result<(), TargetRepositoryError> {
        debug!("git remote add {} {}", name, url);
        Command::new("git")
            .args(&["remote", "add", name, url])
            .current_dir(&self.path)
            .status()?;
        Ok(())
    }

    fn checkout(&self, branch: &str) -> Result<(), TargetRepositoryError> {
        debug!("git checkout -B {}", branch);
        Command::new("git")
            .args(&["checkout", "-B", branch])
            .current_dir(&self.path)
            .status()?;
        Ok(())
    }

    fn merge_unrelated(&self, branches: &[&str]) -> Result<(), TargetRepositoryError> {
        debug!(
            "git merge -n --allow-unrelated-histories --no-edit {}",
            branches.join(" ")
        );
        Command::new("git")
            .args(&["merge", "-n", "--allow-unrelated-histories", "--no-edit"])
            .args(branches)
            .current_dir(&self.path)
            .status()?;
        Ok(())
    }

    fn fetch_all(&self) -> Result<(), TargetRepositoryError> {
        debug!("git fetch -q --all");
        Command::new("git")
            .args(&["fetch", "-q", "--all"])
            .current_dir(&self.path)
            .status()?;

        debug!("git fetch -q --tags");
        Command::new("git")
            .args(&["fetch", "-q", "--tags"])
            .current_dir(&self.path)
            .status()?;
        Ok(())
    }
}
