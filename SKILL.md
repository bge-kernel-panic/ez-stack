---
name: ez-workflow
description: Use when about to run git branching, commit, push, or PR operations in a repo managed by ez-stack. Provides the complete command reference and agent-specific usage patterns.
---

# ez-stack Workflow Skill

ez-stack makes version control simple for AI coding agents. It replaces raw git/gh commands with higher-level operations that auto-restack, auto-detect problems, and emit structured receipts so agents can verify what happened.

**When to use ez:** If `.git/ez/stack.json` exists in the repo, ALL git branching, committing, pushing, and PR operations MUST go through `ez`.

## Hard Rules

**NEVER use these raw commands when ez is initialized:**

| Instead of | Use |
|------------|-----|
| `git checkout -b` | `ez create <name>` |
| `git commit` | `ez commit -m "..."` |
| `git push` | `ez push` or `ez submit` |
| `gh pr create` | `ez push` (creates PR automatically) |
| `git diff main...HEAD` | `ez diff` |
| `git branch` | `ez branch` |

## Agent Startup Sequence

**When you are dropped into a repo, follow this sequence before doing any work:**

```bash
# 1. Check if ez is initialized
test -f .git/ez/stack.json && echo "ez-managed" || echo "not ez-managed"

# 2. If ez-managed, see what's already going on
ez branch                    # Shows all branches, PRs, worktree paths
ez worktree list             # Shows which worktrees exist

# 3. Create your own isolated worktree — ALWAYS do this
#    Pick a descriptive name for your task
cd $(ez worktree create my-task-name --from main)

# 4. You are now in .worktrees/my-task-name with your own branch
#    No other agent can interfere with your work here
#    The main repo directory is untouched
```

**Why worktrees?** If multiple agents (or you in multiple terminals) work in the same directory, they overwrite each other's files, create merge conflicts, and corrupt each other's staged changes. A worktree gives each agent a completely isolated copy of the repo with its own branch. All worktrees share the same `.git` so branches and state are synchronized.

**Setting up the worktree:** Your worktree is a full working copy. If the project needs setup (install deps, build, etc.), run that in your worktree:

```bash
# Example: Node.js project
npm install

# Example: Rust project
cargo build

# Example: Python project
pip install -e .
```

## Working in Your Worktree

### Make incremental, focused changes

Each branch should be ONE logical change. If your task involves multiple independent pieces, stack them:

```bash
# First piece: data types
ez commit -m "feat: add auth types" -- src/auth/types.rs src/auth/mod.rs

# Second piece stacks on top: API routes that use those types
ez create feat/auth-api
# ... make changes ...
ez commit -m "feat: add login/logout API" -- src/auth/api.rs

# Third piece: middleware
ez create feat/auth-middleware
# ... make changes ...
ez commit -m "feat: add auth middleware" -- src/middleware/auth.rs
```

**Scoping rule:** Only commit files that belong to your current task. If you notice a bug in an unrelated file, DON'T fix it on this branch. Instead:
```bash
# Create a separate branch from main for the unrelated fix
ez create fix/typo-in-readme --from main
ez commit -m "fix: typo in README" -- README.md
ez push --title "fix: typo in README"
# Switch back to your task
ez checkout feat/my-task
```

### Self-review before pushing

```bash
ez diff --stat              # What files changed vs parent
ez diff --name-only         # Just the file list
ez diff                     # Full diff (what the PR reviewer sees)
```

### Push and create PRs

```bash
# Push just this branch
ez push --title "feat: add auth types"

# Or push the entire stack at once (creates PRs for all branches)
ez submit
```

### Stay in sync with other agents' work

```bash
# Sync pulls latest main, cleans up merged PRs, restacks your branches
ez sync --autostash

# If sync reports redundant_commits > 0, another agent's PR landed changes
# that overlap with yours — ez auto-dropped the duplicate commits
```

### When you're done

```bash
# Delete your worktree and go back to repo root
cd $(ez worktree delete my-task-name --yes)
```

Or just leave it — `ez sync` will clean up worktrees for merged branches automatically.

## Multi-Agent Rules

- **One worktree per agent.** Never share a worktree.
- **Always create from main** unless explicitly stacking on another branch.
- **Sync before push** to pick up other agents' merged work.
- **Check `ez branch` before starting** to avoid duplicate work.
- **Commit specific files** (`-- path1 path2`) not `-a` to avoid staging unintended changes.
- **Don't touch the main repo directory.** Work only in your `.worktrees/` directory.

## Command Reference

### Branching

| Intent | Command |
|--------|---------|
| Create stacked branch | `ez create <name>` |
| Create and commit | `ez create <name> -m "msg"` |
| Create and stage+commit | `ez create <name> -am "msg"` |
| Create from specific base (no checkout) | `ez create <name> --from <base>` |
| Switch to branch (cd's to worktree if applicable) | `ez checkout <name>` |
| Switch by PR number | `ez checkout 42` |
| List all branches with PRs and worktree paths | `ez branch` |
| Navigate stack | `ez up` / `ez down` / `ez top` / `ez bottom` |
| Delete branch | `ez delete [branch]` |
| Move branch to new parent | `ez move --onto <branch>` |

### Committing

| Intent | Command |
|--------|---------|
| Commit (restacks children) | `ez commit -m "msg"` |
| Stage all + commit | `ez commit -am "msg"` |
| Multi-paragraph commit | `ez commit -m "subject" -m "body"` |
| Commit specific files only | `ez commit -m "msg" -- path1 path2` |
| No-op if nothing staged | `ez commit -m "msg" --if-changed` |
| Amend last commit | `ez amend` |
| Amend with new message | `ez amend -m "new msg"` |

### Diffing

| Intent | Command |
|--------|---------|
| Full diff vs parent (what PR reviewer sees) | `ez diff` |
| Diffstat summary | `ez diff --stat` |
| Changed file names only | `ez diff --name-only` |
| Parent branch name (pipeable) | `ez parent` |

### Pushing and PRs

| Intent | Command |
|--------|---------|
| Push current branch + create/update PR | `ez push` |
| Push with title/body | `ez push --title "..." --body "..."` |
| Push entire stack | `ez submit` |
| Print PR URL to stdout | `ez pr-link` |
| Edit PR title/body | `ez pr-edit --title "..." --body "..."` |
| Mark PR as draft / ready | `ez draft` / `ez ready` |

### Syncing

| Intent | Command |
|--------|---------|
| Sync with trunk (fetch, clean merged, restack) | `ez sync` |
| Sync with dirty working tree | `ez sync --autostash` |
| Preview what sync would do | `ez sync --dry-run` |
| Restack children onto current branch tip | `ez restack` |

### Inspecting

| Intent | Command |
|--------|---------|
| Stack tree with PR status | `ez log` |
| Stack tree as JSON | `ez log --json` |
| Current branch info | `ez status` |
| Current branch info as JSON | `ez status --json` |

### Worktrees

**All worktrees MUST live under `.worktrees/`.** Never use `git worktree add` directly.

| Intent | Command |
|--------|---------|
| Create worktree + cd into it | `cd $(ez worktree create <name>)` |
| Create from specific base | `cd $(ez worktree create <name> --from main)` |
| Delete worktree (from inside it) | `cd $(ez worktree delete <name> --yes)` |
| Force-delete (discard changes) | `ez worktree delete <name> --force` |
| List worktrees | `ez worktree list` |

### Setup and Updates

| Intent | Command |
|--------|---------|
| Install this skill into the current repo | `ez skill install` |
| Remove this skill from the current repo | `ez skill uninstall` |
| First-time shell setup | `ez setup --yes` |
| Update to latest version | `ez update` |
| Check for updates | `ez update --check` |

## Mutation Receipts

Every command that changes git state emits a JSON receipt to stderr. **Parse these to verify operations instead of running separate commands.**

### Receipt examples

```
{"cmd":"commit","branch":"feat/auth","before":"abc1234","after":"def5678","files_changed":3,"insertions":42,"deletions":7}
{"cmd":"sync","branch":"feat/auth","action":"restacked","parent":"main","before":"abc1234","after":"def5678","redundant_commits":0}
{"cmd":"sync","branch":"feat/old","action":"cleaned","reason":"merged"}
{"cmd":"push","branch":"feat/auth","pr_number":42,"pr_url":"https://github.com/...","created":true}
{"cmd":"create","branch":"feat/new","parent":"main","head":"abc1234"}
```

### What to check

| After | Check | Problem if wrong |
|-------|-------|-----------------|
| commit/amend | `files_changed` matches intent | Accidentally staged unrelated files |
| sync/restack | `redundant_commits` > 0 | Commits already in parent, auto-dropped |
| sync | `action: "cleaned"` | Branch's PR was merged and removed |
| push | `created: true/false` | New PR vs update to existing |
| restack | `before == after` | Restack was a no-op |

### Parsing receipts

```bash
# Receipts are JSON lines on stderr mixed with status messages
# Extract the last receipt:
OUTPUT=$(ez commit -am "msg" 2>&1)
RECEIPT=$(echo "$OUTPUT" | grep '^{' | tail -1)
```

## Output Format

Every command appends timing to stderr: `[ok | 45ms]` or `[exit:3 | 120ms]`

Discovery commands (`ez`, `ez worktree`, `ez <cmd> --help`) always exit 0.

Errors always include what to do next.

## Exit Codes

| Code | Meaning | Action |
|------|---------|--------|
| 0 | Success | Continue |
| 1 | Unexpected error | Log and stop |
| 2 | GitHub API error | Check `gh auth status`, retry |
| 3 | Rebase conflict | Resolve conflicts, `ez restack` |
| 4 | Stale remote ref | `git fetch origin <branch>`, retry |
| 5 | Usage error | Check branch state with `ez status` |
| 6 | Unstaged changes | Use `--autostash` or `--if-changed` |
