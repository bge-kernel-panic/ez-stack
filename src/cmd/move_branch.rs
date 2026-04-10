use anyhow::{Result, bail};

use crate::cmd::rebase_conflict;
use crate::cmd::restack_children;
use crate::error::EzError;
use crate::git;
use crate::github;
use crate::stack::StackState;
use crate::ui;

pub fn run(onto: &str) -> Result<()> {
    let mut state = StackState::load()?;
    if let Some(root) = git::current_linked_worktree_root()? {
        ui::linked_worktree_warning(&root);
    }
    let current = git::current_branch()?;

    if state.is_trunk(&current) {
        bail!(EzError::OnTrunk);
    }

    if !state.is_managed(&current) {
        bail!(EzError::BranchNotInStack(current.clone()));
    }

    // The --onto target must be trunk or a managed branch.
    if !state.is_trunk(onto) && !state.is_managed(onto) {
        bail!(EzError::UserMessage(format!(
            "Target branch `{onto}` is not trunk or a managed branch"
        )));
    }

    // Prevent moving onto self.
    if onto == current {
        bail!(EzError::UserMessage(
            "Cannot move a branch onto itself".to_string()
        ));
    }

    // Prevent moving onto a descendant (would create a cycle).
    let path = state.path_to_trunk(onto);
    if path.contains(&current) {
        bail!(EzError::UserMessage(format!(
            "Cannot move `{current}` onto `{onto}` — `{onto}` is a descendant of `{current}`"
        )));
    }

    let meta = state.get_branch(&current)?;
    let old_parent = meta.parent.clone();
    let old_parent_head = meta.parent_head.clone();
    let pr_number = meta.pr_number;

    let new_parent_head = git::rev_parse(onto)?;

    // Rebase current branch onto the new parent.
    let sp = ui::spinner(&format!("Rebasing `{current}` onto `{onto}`..."));
    let outcome = git::rebase_onto(&new_parent_head, &old_parent_head, &current)?;
    sp.finish_and_clear();

    if let git::RebaseOutcome::Conflict(conflict) = outcome {
        rebase_conflict::report(
            "move",
            &current,
            onto,
            &conflict,
            &format!("ez move --onto {onto}"),
        );
        bail!(EzError::RebaseConflict(current.clone()));
    }

    // Update branch metadata.
    let meta = state.get_branch_mut(&current)?;
    meta.parent = onto.to_string();
    meta.parent_head = new_parent_head;

    // Update PR base if a PR exists.
    if let Some(pr) = pr_number {
        let base = if state.is_trunk(onto) {
            state.trunk.clone()
        } else {
            onto.to_string()
        };
        if let Err(e) = github::update_pr_base(pr, &base) {
            ui::warn(&format!("Failed to update PR base: {e}"));
        }
    }

    let new_tip = git::rev_parse(&current)?;
    restack_children::restack_children(&mut state, &current, &new_tip, "move")?;

    ui::success(&format!("Moved `{current}` onto `{onto}`"));

    ui::receipt(&serde_json::json!({
        "cmd": "move",
        "branch": current,
        "from": old_parent,
        "onto": onto,
    }));

    Ok(())
}
