# Stack Links in PR Bodies — Design Spec

**Date:** 2026-03-10
**Status:** Approved

## Goal

When `ez push` creates a new PR (or updates one with an explicit `--body`), automatically append a **Stack** section to the PR body that links to all upstream ancestor PRs in trunk-to-current order, so reviewers understand the dependency chain at a glance.

Also add `ez push --stack` as a shorthand for `ez submit` (push + PR for entire stack, bottom-to-top).

---

## Behavior

| Scenario | Body behavior |
|---|---|
| New PR, no `--body` | Default body + generated stack section |
| New PR, `--body` provided | User body + generated stack section |
| Existing PR, no `--body` | Body left unchanged |
| Existing PR, `--body` provided | User body + generated stack section, written via `gh pr edit` |
| Ancestor has no PR yet | Skipped in stack section |
| No ancestors with PRs | Stack section omitted entirely |

---

## Stack Section Format

```
Part of a stack managed by `ez`.

---

**Stack:**
1. [feat/auth-types #101](https://github.com/org/repo/pull/101)
2. [feat/auth-api #102](https://github.com/org/repo/pull/102)
```

- Ancestors ordered trunk-closest first
- Only upstream PRs are listed — current branch is NOT shown
- Ancestors with a known PR number are rendered as `[branch #N](url)` links
- Ancestors without PRs are skipped entirely
- If no ancestors have PRs, the `---\n**Stack:**` section is omitted

---

## Architecture

### New file: `src/stack_body.rs`

Pure functions — no git, no gh, no I/O. Fully unit-testable.

```rust
pub struct AncestorPr {
    pub branch: String,
    pub pr_number: Option<u64>,
    pub pr_url: Option<String>,
}

/// Returns the full PR body: user_body + stack section (if any ancestors have PRs).
pub fn build_stack_body(ancestors: &[AncestorPr], current: &str, user_body: &str) -> String

/// Returns only the stack section string, or None if no ancestors have PRs.
pub fn build_stack_section(ancestors: &[AncestorPr], current: &str) -> Option<String>
```

### Unit tests (in `src/stack_body.rs` under `#[cfg(test)]`)

- No ancestors → `build_stack_section` returns `None`, body unchanged
- One ancestor with PR → section present with markdown link
- One ancestor without PR → skipped, section returns `None`
- Mixed ancestors (some with PRs, some without) → only linked ones appear
- Custom user body preserved above section, separated by `\n\n---\n\n`
- Current branch NOT shown in section (only upstream ancestors)
- All ancestors missing PRs → section omitted

### Changes to `src/cmd/push.rs` — `push_or_update_pr`

1. Accept `body_explicitly_set: bool` parameter (true when `--body` or `--body-file` was passed)
2. Collect ancestors: `state.path_to_trunk(branch)` reversed, drop trunk and current branch
3. For each ancestor, read `pr_number` from state; if Some, call `github::get_pr_status` to get URL
4. Call `build_stack_body(ancestors, branch, resolved_body)` to get final body
5. **New PR**: pass generated body to `create_pr` (always)
6. **Existing PR + body explicitly set**: call `github::edit_pr(pr.number, None, Some(&generated_body))`
7. **Existing PR, no explicit body**: skip body update entirely

### Changes to `src/cli.rs` — `Push` variant

Add:
```rust
/// Push all branches in the stack (equivalent to ez submit)
#[arg(long)]
stack: bool,
```

### Changes to `src/cmd/push.rs` — `run`

If `stack` is true, delegate to `submit::run(draft, title, body, body_file)` and return.

### Changes to `src/main.rs`

Pass `stack` from the `Push` match arm into `push::run`.

### `src/cmd/mod.rs`

Register `pub mod stack_body;`.

---

## What Does NOT Change

- `ez submit` behavior unchanged
- `ez pr edit` behavior unchanged
- PR body on re-push without `--body` is not touched
