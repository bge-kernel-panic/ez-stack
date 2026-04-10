use anyhow::{Result, bail};

use crate::cmd::rebase_conflict;
use crate::error::EzError;
use crate::git;
use crate::stack::StackState;
use crate::ui;

/// Restack all children of `branch` onto `new_head`.
/// Saves state and returns to `branch` after restacking.
/// Returns the number of children restacked.
pub fn restack_children(
    state: &mut StackState,
    branch: &str,
    new_head: &str,
    cmd_label: &str,
) -> Result<usize> {
    let children = state.children_of(branch);
    if children.is_empty() {
        return Ok(0);
    }

    let current_root = git::repo_root()?;
    let mut restacked = 0;

    for child in &children {
        if let Ok(Some(_wt_path)) = git::branch_checked_out_elsewhere(child, &current_root) {
            ui::info(&format!("Skipped `{child}` (in worktree)"));
            continue;
        }

        let old_base = state.get_branch(child)?.parent_head.clone();

        if old_base == new_head {
            continue;
        }

        let sp = ui::spinner(&format!("Restacking `{child}`..."));
        let outcome = git::rebase_onto(new_head, &old_base, child)?;
        sp.finish_and_clear();

        match outcome {
            git::RebaseOutcome::RebasingComplete => {
                let meta = state.get_branch_mut(child)?;
                meta.parent_head = new_head.to_string();
                restacked += 1;
                ui::info(&format!("Restacked `{child}`"));
            }
            git::RebaseOutcome::Conflict(conflict) => {
                state.save()?;
                git::checkout(branch)?;
                rebase_conflict::report(cmd_label, child, branch, &conflict, "ez restack");
                bail!(EzError::RebaseConflict(child.clone()));
            }
        }
    }

    git::checkout(branch)?;
    state.save()?;

    if restacked > 0 {
        ui::info(&format!("Restacked {restacked} child branch(es)"));
    }

    Ok(restacked)
}
