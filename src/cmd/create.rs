use anyhow::{Result, bail};

use crate::error::RsError;
use crate::git;
use crate::stack::StackState;
use crate::ui;

pub fn run(name: &str, message: Option<&str>) -> Result<()> {
    let mut state = StackState::load()?;
    let current = git::current_branch()?;

    if !state.is_trunk(&current) && !state.is_managed(&current) {
        bail!(RsError::UserMessage(format!(
            "current branch `{current}` is not tracked by rs — switch to a managed branch or trunk first"
        )));
    }

    if git::branch_exists(name) {
        bail!(RsError::BranchAlreadyExists(name.to_string()));
    }

    // If a commit message was provided, stage and commit on the current branch first.
    if let Some(msg) = message {
        if !git::has_staged_changes()? {
            bail!(RsError::NothingToCommit);
        }
        git::commit(msg)?;
        ui::info(&format!("Committed on `{current}`: {msg}"));
    }

    let parent_head = git::rev_parse("HEAD")?;

    git::create_branch(name)?;

    let parent = current;
    state.add_branch(name, &parent, &parent_head);
    state.save()?;

    ui::success(&format!("Created branch `{name}` on top of `{parent}`"));
    Ok(())
}
