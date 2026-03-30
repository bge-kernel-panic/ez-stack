use anyhow::Result;
use std::collections::HashMap;
use std::thread;

use crate::git;
use crate::github;
use crate::stack::StackState;
use crate::ui;

fn dev_port(branch: &str) -> u16 {
    let mut hash: u32 = 5381;
    for byte in branch.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
    }
    10000 + (hash % 10000) as u16
}

fn format_age(secs: Option<u64>) -> String {
    match secs {
        Some(s) if s < 60 => format!("{}s", s),
        Some(s) if s < 3600 => format!("{}m", s / 60),
        Some(s) if s < 86400 => format!("{}h", s / 3600),
        Some(s) => format!("{}d", s / 86400),
        None => "-".to_string(),
    }
}

fn row(m: &str, b: &str, pr: &str, ci: &str, age: &str, port: &str, st: &str) -> String {
    format!("{m:<4} {b:<30} {pr:<8} {ci:<6} {age:<6} {port:<7} {st}")
}

/// Fetched data for one branch.
struct BranchData {
    name: String,
    pr_number: Option<u64>,
    parent: String,
    wt_path: Option<String>,
    ci: String,
    age: Option<u64>,
    wt_status: (usize, usize, usize),
}

pub fn run(json: bool) -> Result<()> {
    let state = StackState::load()?;
    let current = git::current_branch()?;

    let worktree_map: HashMap<String, String> = git::worktree_list()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|wt| wt.branch.map(|b| (b, wt.path)))
        .collect();

    let order = state.topo_order();

    // Collect what we need per branch, then fetch everything in parallel.
    #[allow(clippy::type_complexity)]
    let branch_specs: Vec<(String, Option<u64>, String, Option<String>)> = order
        .iter()
        .filter_map(|b| {
            let meta = state.get_branch(b).ok()?;
            Some((
                b.clone(),
                meta.pr_number,
                meta.parent.clone(),
                worktree_map.get(b.as_str()).cloned(),
            ))
        })
        .collect();

    // One API call for all CI statuses (instead of N sequential gh calls).
    let has_any_pr = branch_specs.iter().any(|(_, pr, _, _)| pr.is_some());
    let ci_handle = thread::spawn(move || {
        if has_any_pr {
            github::get_all_ci_statuses()
        } else {
            HashMap::new()
        }
    });

    // Parallel git calls: age + working tree status per branch.
    let git_handles: Vec<_> = branch_specs
        .iter()
        .map(|(name, _pr_num, _parent, wt_path)| {
            let name = name.clone();
            let wt = wt_path.clone();
            thread::spawn(move || {
                let age = git::log_oneline_time(&name);
                let wt_status = wt
                    .as_ref()
                    .map(|p| git::working_tree_status_at(p))
                    .unwrap_or((0, 0, 0));
                (age, wt_status)
            })
        })
        .collect();

    // Trunk age (runs in parallel with the above).
    let trunk_age = format_age(git::log_oneline_time(&state.trunk));

    // Collect results.
    let ci_map = ci_handle.join().unwrap_or_default();
    let git_results: Vec<(Option<u64>, (usize, usize, usize))> = git_handles
        .into_iter()
        .map(|h| h.join().unwrap_or((None, (0, 0, 0))))
        .collect();

    // Merge into final results.
    #[allow(clippy::type_complexity)]
    let results: Vec<(String, Option<u64>, (usize, usize, usize))> = branch_specs
        .iter()
        .enumerate()
        .map(|(i, (name, _, _, _))| {
            let ci = ci_map.get(name.as_str()).cloned().unwrap_or_default();
            let (age, wt_status) = git_results[i];
            (ci, age, wt_status)
        })
        .collect();

    let branch_data: Vec<BranchData> = branch_specs
        .into_iter()
        .zip(results)
        .map(
            |((name, pr_number, parent, wt_path), (ci, age, wt_status))| BranchData {
                name,
                pr_number,
                parent,
                wt_path,
                ci,
                age,
                wt_status,
            },
        )
        .collect();

    if json {
        return run_json(&state, &current, &branch_data);
    }

    // Render table.
    eprintln!("{}", row("", "BRANCH", "PR", "CI", "AGE", "PORT", "STATUS"));
    eprintln!("{}", "-".repeat(80));

    let m = if current == state.trunk { " *" } else { "  " };
    let trunk_label = format!("{} (trunk)", state.trunk);
    eprintln!("{}", row(m, &trunk_label, "-", "-", &trunk_age, "-", "-"));

    for b in &branch_data {
        let m = if b.name == current { " *" } else { "  " };
        let pr = b.pr_number.map(|n| format!("#{n}")).unwrap_or("-".into());
        let ci = if b.ci.is_empty() { "-" } else { &b.ci };
        let age = format_age(b.age);
        let has_wt = b.wt_path.is_some();
        let port = if has_wt {
            format!("{}", dev_port(&b.name))
        } else {
            "-".into()
        };
        let (s, mo, u) = b.wt_status;
        let status: String = if has_wt {
            if s == 0 && mo == 0 && u == 0 {
                "clean".into()
            } else {
                let mut p = Vec::new();
                if s > 0 {
                    p.push(format!("{s}S"));
                }
                if mo > 0 {
                    p.push(format!("{mo}M"));
                }
                if u > 0 {
                    p.push(format!("{u}U"));
                }
                p.join(" ")
            }
        } else {
            "no worktree".into()
        };

        eprintln!("{}", row(m, &b.name, &pr, ci, &age, &port, &status));
    }

    if current != state.trunk && !state.is_managed(&current) {
        let age = format_age(git::log_oneline_time(&current));
        eprintln!(
            "{}",
            row(" *", &current, "-", "-", &age, "-", "not tracked")
        );
        ui::hint("use `ez create` to track branches");
    }

    Ok(())
}

fn run_json(state: &StackState, current: &str, branches: &[BranchData]) -> Result<()> {
    let mut entries = Vec::new();

    entries.push(serde_json::json!({
        "branch": state.trunk,
        "is_trunk": true,
        "is_current": current == state.trunk,
    }));

    for b in branches {
        let has_wt = b.wt_path.is_some();
        let (s, m, u) = b.wt_status;
        let wt_status = if has_wt {
            Some(serde_json::json!({"staged": s, "modified": m, "untracked": u}))
        } else {
            None
        };
        let ci = if b.ci.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(b.ci.clone())
        };

        entries.push(serde_json::json!({
            "branch": b.name,
            "is_current": b.name == current,
            "parent": b.parent,
            "pr_number": b.pr_number,
            "ci_status": ci,
            "last_activity_secs": b.age,
            "dev_port": if has_wt { Some(dev_port(&b.name)) } else { None },
            "worktree_path": b.wt_path,
            "working_tree": wt_status,
        }));
    }

    println!("{}", serde_json::to_string_pretty(&entries)?);
    Ok(())
}
