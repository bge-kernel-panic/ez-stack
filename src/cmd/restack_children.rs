use anyhow::{Result, bail};

use crate::cmd::rebase_conflict;
use crate::error::EzError;
use crate::git;
use crate::stack::StackState;
use crate::ui;

/// Restack every descendant of `branch` onto its parent's current tip.
///
/// Walks the whole subtree, not just direct children: when a child is rebased
/// its tip moves, which invalidates the `parent_head` of its own children, so
/// we have to propagate the new tip down the stack.
///
/// Saves state and returns to `branch` after restacking. Returns the number of
/// branches that were actually rebased.
pub fn restack_children(
    state: &mut StackState,
    branch: &str,
    new_head: &str,
    cmd_label: &str,
) -> Result<usize> {
    let current_root = git::repo_root()?;
    let mut restacked = 0;

    // Work stack of (branch_to_restack, new_tip_of_its_parent).
    // We only enqueue a branch after its parent has been processed, so the
    // `parent_tip` value here is always the up-to-date tip to rebase onto.
    let mut work: Vec<(String, String)> = state
        .children_of(branch)
        .into_iter()
        .map(|c| (c, new_head.to_string()))
        .collect();

    if work.is_empty() {
        return Ok(0);
    }

    while let Some((child, parent_tip)) = work.pop() {
        if let Ok(Some(_wt_path)) = git::branch_checked_out_elsewhere(&child, &current_root) {
            ui::info(&format!("Skipped `{child}` (in worktree)"));
            // Don't descend: the child's tip hasn't moved, so its own children
            // are still correctly based on it. If the user later restacks the
            // child, that will cascade to grandchildren at that time.
            continue;
        }

        let old_base = state.get_branch(&child)?.parent_head.clone();

        if old_base == parent_tip {
            // Child is already based on its parent's current tip — its tip
            // didn't move, so grandchildren are still consistent. Skip.
            continue;
        }

        let sp = ui::spinner(&format!("Restacking `{child}`..."));
        let outcome = git::rebase_onto(&parent_tip, &old_base, &child)?;
        sp.finish_and_clear();

        match outcome {
            git::RebaseOutcome::RebasingComplete => {
                let child_new_tip = git::rev_parse(&child)?;
                let meta = state.get_branch_mut(&child)?;
                meta.parent_head = parent_tip.clone();
                restacked += 1;
                ui::info(&format!("Restacked `{child}`"));

                for grandchild in state.children_of(&child) {
                    work.push((grandchild, child_new_tip.clone()));
                }
            }
            git::RebaseOutcome::Conflict(conflict) => {
                state.save()?;
                git::checkout(branch)?;
                let parent_name = state
                    .get_branch(&child)
                    .map(|m| m.parent.clone())
                    .unwrap_or_else(|_| branch.to_string());
                rebase_conflict::report(cmd_label, &child, &parent_name, &conflict, "ez restack");
                bail!(EzError::RebaseConflict(child.clone()));
            }
        }
    }

    git::checkout(branch)?;
    state.save()?;

    if restacked > 0 {
        ui::info(&format!("Restacked {restacked} descendant branch(es)"));
    }

    Ok(restacked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{CwdGuard, init_git_repo, take_env_lock, write_file};

    // Build `main → feat/a → feat/b → feat/c` with one commit on each branch,
    // track all three in StackState, and return the tip SHAs of a, b, c.
    fn build_three_deep_stack() -> (String, String, String) {
        let mut state = StackState::new("main".to_string());
        state.save().expect("save initial state");

        let main_tip = git::rev_parse("main").expect("main tip");

        let cwd = std::env::current_dir().expect("cwd");

        git::create_branch("feat/a").expect("create a");
        write_file(&cwd, "a.txt", "a\n");
        git::add_paths(&["a.txt".to_string()]).expect("stage a");
        git::commit("a commit").expect("commit a");
        let a_tip = git::rev_parse("feat/a").expect("a tip");
        state.add_branch("feat/a", "main", &main_tip, None, None);

        git::create_branch("feat/b").expect("create b");
        write_file(&cwd, "b.txt", "b\n");
        git::add_paths(&["b.txt".to_string()]).expect("stage b");
        git::commit("b commit").expect("commit b");
        let b_tip = git::rev_parse("feat/b").expect("b tip");
        state.add_branch("feat/b", "feat/a", &a_tip, None, None);

        git::create_branch("feat/c").expect("create c");
        write_file(&cwd, "c.txt", "c\n");
        git::add_paths(&["c.txt".to_string()]).expect("stage c");
        git::commit("c commit").expect("commit c");
        let c_tip = git::rev_parse("feat/c").expect("c tip");
        state.add_branch("feat/c", "feat/b", &b_tip, None, None);

        state.save().expect("save state");
        (a_tip, b_tip, c_tip)
    }

    #[test]
    fn restack_children_cascades_through_grandchildren() {
        let _guard = take_env_lock();
        let repo = init_git_repo("restack-cascade");
        let _cwd = CwdGuard::enter(&repo);

        let (a_tip_before, b_tip_before, c_tip_before) = build_three_deep_stack();

        // Simulate `ez commit` on feat/a: add a new commit so a's tip moves.
        git::checkout("feat/a").expect("checkout a");
        write_file(&repo, "a2.txt", "a2\n");
        git::add_paths(&["a2.txt".to_string()]).expect("stage a2");
        git::commit("a second commit").expect("second commit on a");
        let a_tip_after = git::rev_parse("feat/a").expect("a tip after");
        assert_ne!(a_tip_after, a_tip_before, "a's tip should have moved");

        // Act: cascade the restack.
        let mut state = StackState::load().expect("load state");
        let restacked = restack_children(&mut state, "feat/a", &a_tip_after, "commit")
            .expect("restack_children should succeed");

        // Assert: both b and c were restacked.
        assert_eq!(restacked, 2, "should restack both the child and grandchild");

        let b_tip_after = git::rev_parse("feat/b").expect("b tip after");
        let c_tip_after = git::rev_parse("feat/c").expect("c tip after");
        assert_ne!(b_tip_after, b_tip_before, "b's tip should have moved");
        assert_ne!(c_tip_after, c_tip_before, "c's tip should have moved");

        // Assert: parent_head metadata was updated for both descendants.
        let reloaded = StackState::load().expect("reload state");
        assert_eq!(
            reloaded.get_branch("feat/b").unwrap().parent_head,
            a_tip_after,
            "b's parent_head should point to a's new tip"
        );
        assert_eq!(
            reloaded.get_branch("feat/c").unwrap().parent_head,
            b_tip_after,
            "c's parent_head should point to b's new tip",
        );

        // Assert: each branch still carries the commit it started with (rebase,
        // not reset) — its file should be present at the branch tip.
        git::checkout("feat/b").expect("checkout b");
        assert!(repo.join("b.txt").exists(), "b.txt should exist on feat/b");
        git::checkout("feat/c").expect("checkout c");
        assert!(repo.join("c.txt").exists(), "c.txt should exist on feat/c");
    }

    #[test]
    fn restack_children_returns_zero_when_no_children() {
        let _guard = take_env_lock();
        let repo = init_git_repo("restack-nochild");
        let _cwd = CwdGuard::enter(&repo);

        let mut state = StackState::new("main".to_string());
        state.save().expect("save state");
        let main_tip = git::rev_parse("main").expect("main tip");

        git::create_branch("feat/solo").expect("create solo");
        state.add_branch("feat/solo", "main", &main_tip, None, None);
        state.save().expect("save state");

        let restacked = restack_children(&mut state, "feat/solo", &main_tip, "commit")
            .expect("should succeed with no children");
        assert_eq!(restacked, 0);
    }
}
