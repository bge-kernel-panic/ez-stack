use anyhow::Result;
use std::collections::HashMap;

use crate::git;
use crate::stack::StackState;
use crate::ui;

pub fn run(json: bool) -> Result<()> {
    let state = StackState::load()?;
    let current = git::current_branch()?;

    // Build branch → worktree path map.
    let worktree_map: HashMap<String, String> = git::worktree_list()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|wt| wt.branch.map(|b| (b, wt.path)))
        .collect();

    if json {
        return run_json(&state, &current, &worktree_map);
    }

    // Human-readable output.

    // Show trunk first.
    let trunk_marker = if current == state.trunk { "* " } else { "  " };
    let trunk_wt = worktree_map
        .get(state.trunk.as_str())
        .map(|p| format!(" {p}"))
        .unwrap_or_default();
    eprintln!("{trunk_marker}{} (trunk){trunk_wt}", state.trunk);

    // Show all managed branches in topo order.
    let order = state.topo_order();
    for branch in &order {
        let meta = state.get_branch(branch)?;
        let marker = if *branch == current { "* " } else { "  " };
        let pr = meta.pr_number.map(|n| format!(" #{n}")).unwrap_or_default();
        let wt = worktree_map
            .get(branch.as_str())
            .map(|p| format!(" {p}"))
            .unwrap_or_default();

        // Show working tree state for branches with worktrees.
        let wt_state = if let Some(wt_path) = worktree_map.get(branch.as_str()) {
            let (staged, modified, untracked) = git::working_tree_status_at(wt_path);
            format_wt_state(staged, modified, untracked)
        } else {
            String::new()
        };

        eprintln!("{marker}{branch}{pr}{wt}{wt_state}");
    }

    // If current branch is not trunk and not managed, show it with a warning.
    if current != state.trunk && !state.is_managed(&current) {
        let wt = worktree_map
            .get(current.as_str())
            .map(|p| format!(" {p}"))
            .unwrap_or_default();
        eprintln!("* {current} (not tracked by ez){wt}");
        ui::hint(&format!(
            "`{current}` was created outside ez — use `ez create` to track branches"
        ));
    }

    Ok(())
}

fn format_wt_state(staged: usize, modified: usize, untracked: usize) -> String {
    let mut parts = Vec::new();
    if staged > 0 {
        parts.push(format!("{staged} staged"));
    }
    if modified > 0 {
        parts.push(format!("{modified} modified"));
    }
    if untracked > 0 {
        parts.push(format!("{untracked} untracked"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" [{}]", parts.join(", "))
    }
}

fn run_json(
    state: &StackState,
    current: &str,
    worktree_map: &HashMap<String, String>,
) -> Result<()> {
    let mut entries = Vec::new();

    // Trunk entry.
    let trunk_wt = worktree_map.get(state.trunk.as_str());
    let trunk_status = trunk_wt.map(|p| {
        let (s, m, u) = git::working_tree_status_at(p);
        serde_json::json!({"staged": s, "modified": m, "untracked": u})
    });
    entries.push(serde_json::json!({
        "branch": state.trunk,
        "is_trunk": true,
        "is_current": current == state.trunk,
        "worktree_path": trunk_wt,
        "working_tree": trunk_status,
    }));

    // Managed branches in topo order.
    let order = state.topo_order();
    for branch in &order {
        let meta = state.get_branch(branch)?;
        let wt_path = worktree_map.get(branch.as_str());
        let wt_status = wt_path.map(|p| {
            let (s, m, u) = git::working_tree_status_at(p);
            serde_json::json!({"staged": s, "modified": m, "untracked": u})
        });
        entries.push(serde_json::json!({
            "branch": branch,
            "is_trunk": false,
            "is_current": *branch == current,
            "parent": meta.parent,
            "pr_number": meta.pr_number,
            "worktree_path": wt_path,
            "working_tree": wt_status,
        }));
    }

    // Untracked current branch.
    if current != state.trunk && !state.is_managed(current) {
        let wt_path = worktree_map.get(current);
        let wt_status = wt_path.map(|p| {
            let (s, m, u) = git::working_tree_status_at(p);
            serde_json::json!({"staged": s, "modified": m, "untracked": u})
        });
        entries.push(serde_json::json!({
            "branch": current,
            "is_trunk": false,
            "is_current": true,
            "tracked": false,
            "worktree_path": wt_path,
            "working_tree": wt_status,
        }));
    }

    println!("{}", serde_json::to_string_pretty(&entries)?);
    Ok(())
}
