use anyhow::{Result, bail};

use crate::cmd::restack_children;
use crate::error::EzError;
use crate::git;
use crate::stack::StackState;
use crate::ui;

pub fn run(message: Option<&str>, all: bool, verbose: bool) -> Result<()> {
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

    if all {
        git::add_all()?;
    }

    if !all && !git::has_staged_changes()? {
        bail!(EzError::UserMessage(
            "no staged changes to amend\n  → Stage files with `git add <files>`, or use `ez amend -a` to stage all".to_string()
        ));
    }

    let before = git::rev_parse("HEAD")?;

    if let Some(msg) = message {
        git::commit_amend(msg)?;
    } else if !git::commit_amend_interactive(verbose)? {
        return Ok(());
    }

    let after = git::rev_parse("HEAD")?;
    let short_after = &after[..after.len().min(7)];
    ui::success(&format!("Amended commit {short_after}"));

    // Show diff stat so agents can verify what was amended.
    let (files, ins, del) = git::diff_stat_numbers();
    if let Ok(stat) = git::show_stat_head() {
        let stat = stat.trim();
        if !stat.is_empty() {
            eprintln!("{stat}");
        }
    }

    // Emit receipt.
    ui::receipt(&serde_json::json!({
        "cmd": "amend",
        "branch": current,
        "before": &before[..before.len().min(7)],
        "after": short_after,
        "files_changed": files,
        "insertions": ins,
        "deletions": del,
    }));

    restack_children::restack_children(&mut state, &current, &after, "amend")?;

    Ok(())
}
