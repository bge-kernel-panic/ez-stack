use std::path::{Path, PathBuf};

use crate::git;
use crate::ui;

/// Hook files live in `.ez/hooks/<event>/` in the main worktree root.
/// They are markdown files with instructions for agents, NOT executable scripts.
///
/// Directory structure:
///   .ez/hooks/
///     post-create/
///       default.md       ← runs unless --hook overrides
///       setup-node.md    ← ez create --hook setup-node
///       setup-python.md  ← ez create --hook setup-python
///     pre-push/
///       default.md
///
/// ez prints the hook contents to stderr. The agent reads and follows them.
fn hooks_dir() -> Option<PathBuf> {
    let root = git::main_worktree_root().ok()?;
    Some(hooks_dir_from_root(&root))
}

fn hooks_dir_from_root(root: &str) -> PathBuf {
    Path::new(root).join(".ez/hooks")
}

fn hook_path(root: &str, event: &str, hook_name: Option<&str>) -> PathBuf {
    let name = hook_name.unwrap_or("default");
    hooks_dir_from_root(root)
        .join(event)
        .join(format!("{name}.md"))
}

fn list_hook_names(dir: &Path) -> Vec<String> {
    if !dir.exists() {
        return vec![];
    }

    std::fs::read_dir(dir)
        .ok()
        .map(|entries| {
            let mut hooks: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    name.strip_suffix(".md").map(|n| n.to_string())
                })
                .collect();
            hooks.sort();
            hooks
        })
        .unwrap_or_default()
}

/// Get hook content for a specific event and optional hook name.
/// If hook_name is None, looks for "default.md".
/// If hook_name is Some, looks for "<name>.md".
pub fn get_hook(event: &str, hook_name: Option<&str>) -> Option<String> {
    let root = git::main_worktree_root().ok()?;
    let hook_path = hook_path(&root, event, hook_name);

    if !hook_path.exists() {
        return None;
    }

    std::fs::read_to_string(&hook_path).ok()
}

/// Print hook instructions to stderr if the hook file exists.
/// Returns true if a hook was found and printed.
pub fn emit_hook(event: &str, hook_name: Option<&str>) -> bool {
    let name = hook_name.unwrap_or("default");
    if let Some(content) = get_hook(event, hook_name) {
        let content = content.trim();
        if content.is_empty() {
            return false;
        }
        if hook_name.is_some() {
            ui::info(&format!("Hook: {event}/{name}"));
        } else {
            ui::info(&format!("Hook: {event}"));
        }
        eprintln!("{content}");
        true
    } else {
        false
    }
}

/// List available hooks for an event.
pub fn list_hooks(event: &str) -> Vec<String> {
    let dir = match hooks_dir() {
        Some(d) => d.join(event),
        None => return vec![],
    };
    list_hook_names(&dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "ez-hooks-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).expect("create temp dir");
        base
    }

    #[test]
    fn hooks_dir_from_root_appends_expected_path() {
        assert_eq!(
            hooks_dir_from_root("/repo"),
            PathBuf::from("/repo/.ez/hooks")
        );
    }

    #[test]
    fn hook_path_uses_default_when_name_missing() {
        assert_eq!(
            hook_path("/repo", "post-create", None),
            PathBuf::from("/repo/.ez/hooks/post-create/default.md")
        );
        assert_eq!(
            hook_path("/repo", "post-create", Some("setup-node")),
            PathBuf::from("/repo/.ez/hooks/post-create/setup-node.md")
        );
    }

    #[test]
    fn list_hook_names_returns_sorted_markdown_stems_only() {
        let dir = temp_dir("list");
        std::fs::write(dir.join("b.md"), "").expect("write b");
        std::fs::write(dir.join("a.md"), "").expect("write a");
        std::fs::write(dir.join("notes.txt"), "").expect("write notes");

        assert_eq!(
            list_hook_names(&dir),
            vec!["a".to_string(), "b".to_string()]
        );

        let _ = std::fs::remove_dir_all(dir);
    }
}
