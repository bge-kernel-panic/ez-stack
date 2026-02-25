use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum RsError {
    #[error("not a git repository (or any parent up to mount point)")]
    NotARepo,

    #[error("rs is not initialized in this repo — run `rs init` first")]
    NotInitialized,

    #[error("rs is already initialized in this repo")]
    AlreadyInitialized,

    #[error("currently on trunk branch — create a stacked branch first with `rs create <name>`")]
    OnTrunk,

    #[error("branch `{0}` not found in stack metadata")]
    BranchNotInStack(String),

    #[error("branch `{0}` already exists")]
    BranchAlreadyExists(String),

    #[error("no children to restack")]
    NoChildren,

    #[error("already at the top of the stack")]
    AlreadyAtTop,

    #[error("already at the bottom of the stack")]
    AlreadyAtBottom,

    #[error(
        "rebase conflict on branch `{0}` — resolve conflicts, then run `rs restack` to continue"
    )]
    RebaseConflict(String),

    #[error("no staged changes to commit")]
    NothingToCommit,

    #[error("git command failed: {0}")]
    GitError(String),

    #[error("gh CLI error: {0}")]
    GhError(String),

    #[error("{0}")]
    UserMessage(String),
}
