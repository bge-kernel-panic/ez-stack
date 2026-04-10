use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub create: CreateConfig,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CreateConfig {
    /// Whether `ez create` defaults to worktree mode. Default: true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree: Option<bool>,
}

impl Config {
    /// Load config from `~/.ez/config.json`. Returns default if missing or unreadable.
    pub fn load() -> Config {
        let Some(path) = config_path() else {
            return Config::default();
        };
        let Ok(contents) = std::fs::read_to_string(&path) else {
            return Config::default();
        };
        serde_json::from_str(&contents).unwrap_or_default()
    }

    /// Whether `ez create` should use worktree mode by default.
    pub fn create_worktree(&self) -> bool {
        self.create.worktree.unwrap_or(true)
    }
}

fn config_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".ez/config.json"))
}

/// Write the `create.worktree` setting to `~/.ez/config.json`.
pub fn set_create_worktree(value: bool) -> anyhow::Result<()> {
    let path = config_path()
        .ok_or_else(|| anyhow::anyhow!("could not determine home directory ($HOME not set)"))?;

    let mut config = if let Ok(contents) = std::fs::read_to_string(&path) {
        serde_json::from_str(&contents).unwrap_or_default()
    } else {
        Config::default()
    };

    config.create.worktree = Some(value);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_enables_worktree() {
        let config = Config::default();
        assert!(config.create_worktree());
    }

    #[test]
    fn parse_json_worktree_false() {
        let json = r#"{"create": {"worktree": false}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(!config.create_worktree());
    }

    #[test]
    fn parse_empty_json() {
        let config: Config = serde_json::from_str("{}").unwrap();
        assert!(config.create_worktree());
    }

    #[test]
    fn parse_json_worktree_true() {
        let json = r#"{"create": {"worktree": true}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.create_worktree());
    }
}
