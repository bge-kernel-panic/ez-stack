use anyhow::{Result, bail};
use std::process::Command;

use crate::ui;

const REPO: &str = "rohoswagger/ez-stack";

/// Fetch the latest release tag from GitHub API using curl.
fn fetch_latest_version() -> Result<String> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            &format!("https://api.github.com/repos/{REPO}/releases/latest"),
        ])
        .output()?;

    if !output.status.success() {
        bail!(
            "failed to fetch latest version from GitHub\n  → Check your internet connection and try again"
        );
    }

    let body = String::from_utf8_lossy(&output.stdout);

    // Parse tag_name from JSON without a JSON dependency.
    // Format: "tag_name": "v0.1.11"
    for line in body.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("\"tag_name\"") {
            // rest is something like: : "v0.1.11",
            if let Some(start) = rest.find('"') {
                let after = &rest[start + 1..];
                if let Some(end) = after.find('"') {
                    return Ok(after[..end].to_string());
                }
            }
        }
    }

    bail!(
        "could not parse latest version from GitHub API response\n  → Check https://github.com/{REPO}/releases manually"
    );
}

/// Detect whether the current binary was installed via cargo or the install script.
fn detect_install_method() -> InstallMethod {
    let exe = std::env::current_exe().ok();
    if let Some(path) = exe {
        let path_str = path.to_string_lossy();
        // cargo install puts binaries in .cargo/bin/
        if path_str.contains(".cargo/bin") {
            return InstallMethod::Cargo;
        }
    }
    // Default to script-based install (covers ~/.local/bin and custom paths).
    InstallMethod::Script
}

enum InstallMethod {
    Cargo,
    Script,
}

pub fn run(target_version: Option<&str>, check_only: bool) -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    let current_tag = format!("v{current}");

    let sp = ui::spinner("Checking for updates...");
    let latest = fetch_latest_version()?;
    sp.finish_and_clear();

    // Machine-readable: print version info to stdout.
    println!("{current_tag}");

    let target = target_version.unwrap_or(&latest);

    if target == current_tag && target_version.is_none() {
        ui::success(&format!("Already on the latest version ({current_tag})"));
        return Ok(());
    }

    if check_only {
        if target != current_tag {
            ui::info(&format!(
                "Update available: {current_tag} → {target}\n  → Run `ez update` to install"
            ));
        }
        return Ok(());
    }

    ui::info(&format!("Updating {current_tag} → {target}"));

    match detect_install_method() {
        InstallMethod::Cargo => {
            ui::info("Detected cargo install — running `cargo install ez-stack`");
            let mut args = vec!["install", "ez-stack", "--force"];
            // For a specific version, pass --version.
            let ver_str;
            if let Some(v) = target_version {
                ver_str = v.strip_prefix('v').unwrap_or(v).to_string();
                args.push("--version");
                args.push(&ver_str);
            }
            let status = Command::new("cargo").args(&args).status()?;
            if !status.success() {
                bail!("cargo install failed\n  → Try manually: cargo install ez-stack --force");
            }
        }
        InstallMethod::Script => {
            ui::info("Running install script from GitHub");
            let script_url = format!("https://raw.githubusercontent.com/{REPO}/main/install.sh");
            let status = Command::new("bash")
                .args([
                    "-c",
                    &format!("curl -fsSL '{script_url}' | bash -s -- {target}"),
                ])
                .status()?;
            if !status.success() {
                bail!(
                    "install script failed\n  → Try manually: curl -fsSL {script_url} | bash -s -- {target}"
                );
            }
        }
    }

    ui::success(&format!("Updated to {target}"));
    Ok(())
}
