use anyhow::{Result, bail};

use crate::error::RsError;
use crate::git;
use crate::stack::StackState;
use crate::ui;

pub fn run(trunk: Option<String>) -> Result<()> {
    if !git::is_repo() {
        bail!(RsError::NotARepo);
    }

    if StackState::is_initialized()? {
        bail!(RsError::AlreadyInitialized);
    }

    let trunk = match trunk {
        Some(t) => t,
        None => git::default_branch()?,
    };

    let state = StackState::new(trunk.clone());
    state.save()?;

    ui::success(&format!("Initialized rs with trunk branch `{trunk}`"));
    Ok(())
}
