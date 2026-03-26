use anyhow::{Result, bail};
use std::path::PathBuf;

use crate::ui;

/// Returns the user-level ez config directory (~/.ez/).
fn ez_home() -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("could not determine home directory ($HOME not set)"))?;
    Ok(PathBuf::from(home).join(".ez"))
}

/// Returns true if `ez setup` has already been run.
pub fn is_setup_done() -> bool {
    ez_home()
        .map(|p| p.join(".setup-done").exists())
        .unwrap_or(false)
}

fn mark_setup_done() -> Result<()> {
    let dir = ez_home()?;
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(".setup-done"), "")?;
    Ok(())
}

fn detect_shell() -> Option<String> {
    std::env::var("SHELL").ok().and_then(|s| {
        let name = s.rsplit('/').next().unwrap_or(&s).to_string();
        match name.as_str() {
            "bash" | "zsh" | "fish" => Some(name),
            _ => None,
        }
    })
}

fn rc_file_for(shell: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let home = PathBuf::from(home);
    match shell {
        "zsh" => Some(home.join(".zshrc")),
        "bash" => {
            // Prefer .bashrc, fall back to .bash_profile on macOS.
            let bashrc = home.join(".bashrc");
            if bashrc.exists() {
                Some(bashrc)
            } else {
                Some(home.join(".bash_profile"))
            }
        }
        "fish" => Some(home.join(".config/fish/config.fish")),
        _ => None,
    }
}

fn shell_init_line(shell: &str) -> String {
    match shell {
        "fish" => "ez shell-init | source".to_string(),
        _ => r#"eval "$(ez shell-init)""#.to_string(),
    }
}

fn path_export_line(shell: &str, install_dir: &str) -> String {
    match shell {
        "fish" => format!("fish_add_path {install_dir}"),
        _ => format!(r#"export PATH="{install_dir}:$PATH""#),
    }
}

pub fn run(yes: bool) -> Result<()> {
    let shell = detect_shell();

    let Some(shell) = shell else {
        bail!(
            "could not detect shell from $SHELL\n  → Set $SHELL or manually add `eval \"$(ez shell-init)\"` to your shell config"
        );
    };

    let Some(rc_path) = rc_file_for(&shell) else {
        bail!(
            "could not determine rc file for shell `{shell}`\n  → Manually add `eval \"$(ez shell-init)\"` to your shell config"
        );
    };

    let rc_display = rc_path
        .to_str()
        .unwrap_or("shell config")
        .replace(&std::env::var("HOME").unwrap_or_default(), "~");

    // Read existing rc file content (or empty if it doesn't exist).
    let rc_content = std::fs::read_to_string(&rc_path).unwrap_or_default();

    let mut lines_to_add: Vec<String> = Vec::new();

    // Check if PATH needs updating (binary not in a standard PATH location).
    let exe_path = std::env::current_exe().ok();
    if let Some(ref exe) = exe_path {
        if let Some(install_dir) = exe.parent().and_then(|p| p.to_str()) {
            let in_path = std::env::var("PATH")
                .unwrap_or_default()
                .split(':')
                .any(|p| p == install_dir);
            let already_in_rc = rc_content.contains(install_dir);
            if !in_path && !already_in_rc {
                lines_to_add.push(path_export_line(&shell, install_dir));
            }
        }
    }

    // Check if shell-init is already configured.
    let init_line = shell_init_line(&shell);
    let already_has_init = rc_content.contains("ez shell-init");
    if !already_has_init {
        lines_to_add.push(init_line.clone());
    }

    if lines_to_add.is_empty() {
        ui::success(&format!("Shell already configured in {rc_display}"));
        mark_setup_done()?;
        return Ok(());
    }

    // Show what we'll add.
    ui::info(&format!("Will add to {rc_display}:"));
    for line in &lines_to_add {
        eprintln!("  {line}");
    }

    if !yes && !ui::confirm("Add these lines?") {
        ui::info("Cancelled — add manually if needed");
        return Ok(());
    }

    // Append to rc file.
    let mut append = String::new();
    append.push_str("\n# ez-stack shell integration\n");
    for line in &lines_to_add {
        append.push_str(line);
        append.push('\n');
    }

    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&rc_path)?;
    std::fs::write(&rc_path, format!("{rc_content}{append}"))?;

    mark_setup_done()?;

    ui::success(&format!("Updated {rc_display}"));
    ui::hint(&format!("Restart your shell or run: source {rc_display}"));

    Ok(())
}
