use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use log::{debug, info};

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use crate::error::ErrorKind;

use super::{config, env, MercurialRepo, RepositorySavedState, TargetRepository};

pub fn hg2git<P: AsRef<Path>>(
    repourl: P,
    verify: bool,
    git_active_branches: Option<usize>,
    target: &mut TargetRepository,
    env: &env::Environment,
    repository_config: &config::RepositoryConfig,
) -> Result<(), ErrorKind> {
    debug!("Config: {:?}", repository_config);
    debug!("Environment: {:?}", env);

    let repo = MercurialRepo::open(repourl.as_ref(), repository_config, env)?;

    if !repo.verify_heads(repository_config.allow_unnamed_heads)? {
        return Err(ErrorKind::VerifyFailure("Verify heads failed".into()));
    };

    let tip = repo.changelog_len()?;

    let to = if let Some(limit_high) = repository_config.limit_high {
        tip.min(limit_high)
    } else {
        tip
    };

    debug!("Checking saved state...");
    let mut brmap = repository_config
        .branches
        .clone()
        .unwrap_or_else(HashMap::new);
    let mut counter: usize = 0;
    let offset = repository_config.offset.unwrap_or(0);

    let mut errors = None;
    let from_tag =
        {
            let (output, saved_state) = target.start_import(git_active_branches, env.clean)?;

            let (from, from_tag) = if let Some(saved_state) = saved_state.as_ref() {
                match saved_state {
                    RepositorySavedState::OffsetedRevision(rev, from_tag) => {
                        (rev - offset, from_tag - offset)
                    }
                }
            } else {
                (0, 0)
            };

            info!("Exporting commits from {}", from);

            let start = Instant::now();
            let bar = ProgressBar::new((to - from) as u64);
            bar.set_style(ProgressStyle::default_bar().template(
                "{spinner:.green}[{elapsed_precise}] [{wide_bar:.cyan/blue}] {msg} ({eta})",
            ));
            for mut changeset in repo.range(from..to) {
                bar.inc(1);
                bar.set_message(&format!("{:6}/{}", changeset.revision.0, to));
                match repo.export_commit(&mut changeset, counter, &mut brmap, output) {
                    Ok(progress) => counter = progress,
                    x => {
                        errors = Some((x, changeset.revision.0));
                        break;
                    }
                }
            }
            bar.finish_with_message(&format!(
                "Repository {} [{};{}). Elapsed: {}",
                repourl.as_ref().to_str().unwrap(),
                from,
                to,
                HumanDuration(start.elapsed())
            ));

            counter = repo.export_tags(from_tag..to, counter, output)?;
            from_tag
        };

    if let Some((error, at)) = errors {
        if at > 0 {
            info!("Saving last success state...");
            target.save_state(RepositorySavedState::OffsetedRevision(
                at as usize - 1 + offset,
                from_tag + offset,
            ))?;
        }
        error?;
    }

    info!("Issued {} commands", counter);
    info!("Saving state...");
    target.save_state(RepositorySavedState::OffsetedRevision(
        to + offset,
        to + offset,
    ))?;

    target.finish()?;

    if verify {
        target.verify(
            repourl.as_ref().to_str().unwrap(),
            repository_config.path_prefix.as_ref().map(|x| &x[..]),
        )?;
    }

    Ok(())
}
