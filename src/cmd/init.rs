use anyhow::{Result, bail};

use crate::error::EzError;
use crate::git;
use crate::stack::StackState;
use crate::ui;

pub fn run(trunk: Option<String>) -> Result<()> {
    if !git::is_repo() {
        bail!(EzError::NotARepo);
    }

    if StackState::is_initialized()? {
        bail!(EzError::AlreadyInitialized);
    }

    let trunk = match trunk {
        Some(t) => t,
        None => git::default_branch()?,
    };

    let state = StackState::new(trunk.clone());
    state.save()?;

    ui::success(&format!("Initialized ez with trunk branch `{trunk}`"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack::StackState;
    use crate::test_support::{CwdGuard, init_git_repo, take_env_lock, temp_dir};

    #[test]
    fn init_fails_outside_git_repo() {
        let _guard = take_env_lock();
        let dir = temp_dir("init-not-repo");
        let _cwd = CwdGuard::enter(&dir);

        let err = run(None).expect_err("non-repo should fail");
        assert!(matches!(
            err.downcast_ref::<EzError>(),
            Some(EzError::NotARepo)
        ));
    }

    #[test]
    fn init_fails_when_state_already_exists() {
        let _guard = take_env_lock();
        let repo = init_git_repo("init-already");
        let _cwd = CwdGuard::enter(&repo);
        StackState::new("main".to_string())
            .save()
            .expect("save state");

        let err = run(None).expect_err("double init should fail");
        assert!(matches!(
            err.downcast_ref::<EzError>(),
            Some(EzError::AlreadyInitialized)
        ));
    }

    #[test]
    fn init_uses_default_branch_when_trunk_not_provided() {
        let _guard = take_env_lock();
        let repo = init_git_repo("init-default");
        let _cwd = CwdGuard::enter(&repo);

        run(None).expect("init should succeed");
        let state = StackState::load().expect("load state");
        assert_eq!(state.trunk, "main");
        assert_eq!(state.remote, "origin");
    }

    #[test]
    fn init_state_is_visible_from_nested_subdirectory() {
        let _guard = take_env_lock();
        let repo = init_git_repo("init-subdir");
        let _cwd = CwdGuard::enter(&repo);

        run(None).expect("init should succeed");
        std::fs::create_dir_all(repo.join("backend/api")).expect("create nested dirs");
        let _subdir = CwdGuard::enter(&repo.join("backend/api"));

        let state = StackState::load().expect("load state from subdir");
        assert_eq!(state.trunk, "main");
        assert_eq!(state.remote, "origin");
    }
}
