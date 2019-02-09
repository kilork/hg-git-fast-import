use super::read_file;
use super::{config::RepositorySavedState, TargetRepository, TargetRepositoryError};
use std::path::Path;
use std::path::PathBuf;

use log::{error, info};
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::process::{Child, Command, Stdio};

pub struct StdoutTargetRepository<'a> {
    stdoutlock: std::io::StdoutLock<'a>,
}

impl<'a> From<std::io::StdoutLock<'a>> for StdoutTargetRepository<'a> {
    fn from(value: std::io::StdoutLock<'a>) -> Self {
        Self { stdoutlock: value }
    }
}

impl<'a> TargetRepository for StdoutTargetRepository<'a> {
    fn init(
        &mut self,
    ) -> Result<(&mut Write, Option<RepositorySavedState>), TargetRepositoryError> {
        Ok((&mut self.stdoutlock, None))
    }
    fn finish(&mut self) -> Result<(), TargetRepositoryError> {
        Ok(())
    }
}

pub struct GitTargetRepository {
    path: PathBuf,
    fast_import_cmd: Option<Child>,
    saved_state: Option<RepositorySavedState>,
}

impl GitTargetRepository {
    pub fn open<P: AsRef<Path>>(value: P) -> Self {
        Self {
            path: value.as_ref().into(),
            fast_import_cmd: None,
            saved_state: None,
        }
    }

    fn get_saved_state_path(&self) -> PathBuf {
        let mut saved_state = self.path.join(".git").join(env!("CARGO_PKG_NAME"));
        saved_state.set_extension("lock");
        saved_state
    }
}

impl TargetRepository for GitTargetRepository {
    fn init(
        &mut self,
    ) -> Result<(&mut Write, Option<RepositorySavedState>), TargetRepositoryError> {
        let path = &self.path;
        let saved_state;
        info!("Checking Git repo: {}", path.to_str().unwrap());
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
            info!("Creating new dir");
            fs::create_dir_all(path)?;

            info!("Init Git repo");
            let status = Command::new("git").arg("init").current_dir(path).status()?;
            if !status.success() {
                error!("Cannot init Git repo");
                return Err(TargetRepositoryError::CannotInitRepo(status));
            }

            info!("Configure Git repo");
            let status = Command::new("git")
                .args(&["config", "core.ignoreCase", "false"])
                .current_dir(path)
                .status()?;
            if !status.success() {
                error!("Cannot configure Git repo");
                return Err(TargetRepositoryError::CannotConfigRepo(status));
            }

            info!("New Git repo initialization done");
            saved_state = None;
        }

        self.fast_import_cmd = Some(
            Command::new("git")
                .args(&[
                    "fast-import",
                    "--export-marks=.git/hg-git-fast-import.marks",
                    "--import-marks-if-exists=.git/hg-git-fast-import.marks",
                ])
                .current_dir(path)
                .stdin(Stdio::piped())
                .spawn()?,
        );

        Ok((
            self.fast_import_cmd
                .as_mut()
                .map(|x| x.stdin.as_mut().unwrap())
                .unwrap(),
            saved_state,
        ))
    }

    fn finish(&mut self) -> Result<(), TargetRepositoryError> {
        let path = Path::new(&self.path);
        info!("Waiting for Git fast-import to finish");
        let status = self.fast_import_cmd.as_mut().unwrap().wait()?;
        info!("Finished");
        let status = if status.success() {
            info!("Checking out HEAD revision");
            Command::new("git")
                .args(&["checkout", "HEAD"])
                .current_dir(path)
                .spawn()
                .unwrap()
                .wait()
                .unwrap()
        } else {
            error!("Git fast-import failed.");
            return Err(TargetRepositoryError::ImportFailed(status));
        };
        let status = if status.success() {
            info!("Resetting Git repo.");
            Command::new("git")
                .args(&["reset", "--hard"])
                .current_dir(path)
                .spawn()
                .unwrap()
                .wait()
                .unwrap()
        } else {
            panic!("Cannot reset Git repo.")
        };
        let status = if status.success() {
            info!("Cleanup Git repo");
            Command::new("git")
                .args(&["clean", "-d", "-x", "-f"])
                .current_dir(path)
                .spawn()
                .unwrap()
                .wait()
                .unwrap()
        } else {
            panic!("Cannot reset Git repo.")
        };
        if !status.success() {
            panic!("Cannot checkout HEAD revision.");
        };
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
            "Mercurial (source): {} Git(target): {}",
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
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        if status.success() {
            info!("OK")
        } else {
            panic!("Verify failed.");
        }
        Ok(())
    }

    fn get_saved_state(&self) -> Option<&RepositorySavedState> {
        self.saved_state.as_ref()
    }

    fn save_state(&self, state: RepositorySavedState) -> std::io::Result<()> {
        let path = &self.path;
        info!("Saving state to Git repo: {}", path.to_str().unwrap());
        let saved_state_path = self.get_saved_state_path();
        let toml = toml::to_string(&state).unwrap();
        let mut f = File::create(&saved_state_path)?;
        f.write_all(toml.as_bytes())?;
        Ok(())
    }
}
