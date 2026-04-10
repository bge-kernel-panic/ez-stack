use anyhow::{Result, bail};

use crate::error::EzError;
use crate::git;
use crate::stack::StackState;
use crate::ui;

pub fn run(name: Option<&str>, parent: Option<&str>) -> Result<()> {
    let mut state = StackState::load()?;

    // Resolve branch: explicit arg or current branch.
    let name = match name {
        Some(n) => n.to_string(),
        None => git::current_branch()?,
    };
    let name = name.as_str();

    // Branch must exist in git.
    if !git::branch_exists(name) {
        ui::hint(&format!(
            "Use `ez create {name}` to create a new branch instead"
        ));
        bail!(EzError::UserMessage(format!(
            "branch `{name}` does not exist in git"
        )));
    }

    // Branch must not already be tracked.
    if state.is_managed(name) {
        ui::hint("Run `ez log` to see the current stack");
        bail!(EzError::UserMessage(format!(
            "branch `{name}` is already tracked by ez"
        )));
    }

    if state.is_trunk(name) {
        bail!(EzError::UserMessage(format!(
            "cannot adopt trunk branch `{name}`"
        )));
    }

    // Resolve parent: explicit --parent or default to trunk.
    let parent = parent.unwrap_or(&state.trunk).to_string();

    if !state.is_trunk(&parent) && !state.is_managed(&parent) {
        ui::hint("Use trunk or a managed branch as the parent");
        bail!(EzError::UserMessage(format!(
            "parent branch `{parent}` is not tracked by ez"
        )));
    }

    let parent_head = git::rev_parse(&parent)?;
    state.add_branch(name, &parent, &parent_head, None, None);
    state.save()?;

    ui::success(&format!("Adopted `{name}` onto `{parent}`"));

    ui::receipt(&serde_json::json!({
        "cmd": "adopt",
        "branch": name,
        "parent": parent,
        "parent_head": &parent_head[..parent_head.len().min(7)],
    }));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{CwdGuard, init_git_repo, take_env_lock};

    #[test]
    fn adopt_registers_existing_branch() {
        let _guard = take_env_lock();
        let repo = init_git_repo("adopt-basic");
        let _cwd = CwdGuard::enter(&repo);

        let state = StackState::new("main".to_string());
        state.save().expect("save state");

        // Create a branch with raw git (not ez).
        git::create_branch_at("feat/orphan", "main").expect("create branch");

        run(Some("feat/orphan"), None).expect("adopt should succeed");

        let state = StackState::load().expect("reload state");
        assert!(state.is_managed("feat/orphan"));
        let meta = state.get_branch("feat/orphan").expect("branch meta");
        assert_eq!(meta.parent, "main");
    }

    #[test]
    fn adopt_rejects_nonexistent_branch() {
        let _guard = take_env_lock();
        let repo = init_git_repo("adopt-nonexistent");
        let _cwd = CwdGuard::enter(&repo);

        let state = StackState::new("main".to_string());
        state.save().expect("save state");

        let err = run(Some("ghost"), None).expect_err("should fail for nonexistent branch");
        assert!(
            err.to_string().contains("does not exist"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn adopt_rejects_already_tracked_branch() {
        let _guard = take_env_lock();
        let repo = init_git_repo("adopt-already-tracked");
        let _cwd = CwdGuard::enter(&repo);

        let mut state = StackState::new("main".to_string());
        let head = git::rev_parse("main").expect("rev-parse");
        state.add_branch("feat/existing", "main", &head, None, None);
        state.save().expect("save state");

        git::create_branch_at("feat/existing", "main").expect("create branch");

        let err = run(Some("feat/existing"), None).expect_err("should fail for tracked branch");
        assert!(
            err.to_string().contains("already tracked"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn adopt_with_explicit_parent() {
        let _guard = take_env_lock();
        let repo = init_git_repo("adopt-parent");
        let _cwd = CwdGuard::enter(&repo);

        let mut state = StackState::new("main".to_string());
        let head = git::rev_parse("main").expect("rev-parse");
        state.add_branch("feat/base", "main", &head, None, None);
        state.save().expect("save state");

        git::create_branch_at("feat/base", "main").expect("create base branch");
        git::create_branch_at("feat/child", "main").expect("create child branch");

        run(Some("feat/child"), Some("feat/base")).expect("adopt with parent should succeed");

        let state = StackState::load().expect("reload state");
        let meta = state.get_branch("feat/child").expect("branch meta");
        assert_eq!(meta.parent, "feat/base");
    }

    #[test]
    fn adopt_rejects_unmanaged_parent() {
        let _guard = take_env_lock();
        let repo = init_git_repo("adopt-unmanaged-parent");
        let _cwd = CwdGuard::enter(&repo);

        let state = StackState::new("main".to_string());
        state.save().expect("save state");

        git::create_branch_at("random", "main").expect("create random");
        git::create_branch_at("feat/orphan", "main").expect("create orphan");

        let err =
            run(Some("feat/orphan"), Some("random")).expect_err("should fail for unmanaged parent");
        assert!(
            err.to_string().contains("not tracked by ez"),
            "unexpected error: {err:#}"
        );
    }
}
