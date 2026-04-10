use anyhow::{Result, bail};
use std::process::Command;

use crate::ui;

const REPO: &str = "bge-kernel-panic/ez-stack";

fn parse_latest_version_response(body: &str) -> Result<String> {
    for line in body.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("\"tag_name\"") {
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

    parse_latest_version_response(&String::from_utf8_lossy(&output.stdout))
}

/// Detect whether the current binary was installed via cargo or the install script.
fn detect_install_method() -> InstallMethod {
    let exe = std::env::current_exe().ok();
    let exe_path = exe.map(|p| p.to_string_lossy().into_owned());
    detect_install_method_from_path(exe_path.as_deref())
}

fn detect_install_method_from_path(path: Option<&str>) -> InstallMethod {
    if let Some(path_str) = path
        && path_str.contains(".cargo/bin")
    {
        return InstallMethod::Cargo;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_latest_version_response_extracts_tag_name() {
        let body = r#"
        {
          "name": "release",
          "tag_name": "v1.2.3",
          "other": "ignored"
        }"#;
        assert_eq!(
            parse_latest_version_response(body).expect("version"),
            "v1.2.3"
        );
    }

    #[test]
    fn parse_latest_version_response_errors_when_missing() {
        let err = parse_latest_version_response("{}").expect_err("should fail");
        assert!(err.to_string().contains("could not parse latest version"));
    }

    #[test]
    fn detect_install_method_from_path_distinguishes_cargo_installs() {
        assert!(matches!(
            detect_install_method_from_path(Some("/Users/me/.cargo/bin/ez")),
            InstallMethod::Cargo
        ));
        assert!(matches!(
            detect_install_method_from_path(Some("/usr/local/bin/ez")),
            InstallMethod::Script
        ));
        assert!(matches!(
            detect_install_method_from_path(None),
            InstallMethod::Script
        ));
    }
}
