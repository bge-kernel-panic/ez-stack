# ez-stack Advanced Command Reference

This is the full command reference. For the core workflow, see [SKILL.md](SKILL.md).

## Committing

| Intent | Command |
|--------|---------|
| Commit with editor (opens `$EDITOR`) | `ez commit` |
| Commit with editor + diff in buffer | `ez commit -v` |
| Commit with message | `ez commit -m "msg"` |
| Stage all + commit | `ez commit -am "msg"` |
| Multi-paragraph commit | `ez commit -m "subject" -m "body"` |
| Commit specific files only | `ez commit -m "msg" -- path1 path2` |
| No-op if nothing staged | `ez commit -m "msg" --if-changed` |
| Amend with editor (edit existing message) | `ez amend` |
| Amend with editor + diff in buffer | `ez amend -v` |
| Amend with new message | `ez amend -m "new msg"` |
| Stage all + amend (opens editor) | `ez amend -a` |
| Stage all + amend with message | `ez amend -am "new msg"` |

Preferred workflow:

- Focused commit: `ez commit -m "msg" -- path1 path2`
- Bulk update: `ez commit -am "msg"`
- Partial hunks: `git add -p` then `ez commit -m "msg"`
- Edit message interactively: `ez commit` or `ez commit -v`

## Scope Guard

| Intent | Command |
|--------|---------|
| Show current branch scope | `ez scope show` |
| Add patterns to scope | `ez scope add 'src/auth/**' 'tests/auth/**'` |
| Replace scope | `ez scope set --mode strict 'src/auth/**'` |
| Clear scope | `ez scope clear` |

## Diffing and Inspecting

| Intent | Command |
|--------|---------|
| Full diff vs parent | `ez diff` |
| Diffstat summary | `ez diff --stat` |
| Changed file names only | `ez diff --name-only` |
| Parent branch name | `ez parent` |
| Current branch info | `ez status` |
| Current branch info (JSON) | `ez status --json` |
| Stack tree with PR status | `ez log` |
| Stack tree as JSON | `ez log --json` |

## Navigation

| Intent | Command |
|--------|---------|
| Switch to branch | `ez switch <name>` |
| Switch by PR number | `ez switch 42` |
| Move up/down in stack | `ez up` / `ez down` / `ez top` / `ez bottom` |

## PR Management

| Intent | Command |
|--------|---------|
| Print PR URL to stdout | `ez pr-link` |
| Edit PR title/body | `ez pr-edit --title "..." --body "..."` |
| Mark PR as draft / ready | `ez draft` / `ez ready` |
| Merge bottom PR | `ez merge` |
| Merge non-interactively | `ez merge --yes` |
| Merge current linear stack bottom-to-top | `ez merge --stack --yes` |

## Syncing

| Intent | Command |
|--------|---------|
| Sync with trunk | `ez sync` |
| Sync with dirty working tree | `ez sync --autostash` |
| Preview sync | `ez sync --dry-run` |
| Restack children | `ez restack` |

## Stack Operations

| Intent | Command |
|--------|---------|
| Adopt current branch into the stack | `ez adopt` |
| Adopt a named branch | `ez adopt <name>` |
| Adopt with explicit parent | `ez adopt <name> --parent <branch>` |
| Move branch to new parent | `ez move --onto <branch>` |
| Push entire stack | `ez submit` |

## Setup and Maintenance

| Intent | Command |
|--------|---------|
| Install skill in repo | `ez skill install` |
| Shell integration | `ez setup --yes` |
| Default to no-worktree on create | `ez setup --no-worktree` |
| Default to worktree on create | `ez setup --worktree` |
| Update ez | `ez update` |
| Check for updates | `ez update --check` |

## Mutation Receipts

Every mutating command emits JSON to stderr:

| After | Key fields |
|-------|-----------|
| commit/amend | `files_changed`, `insertions`, `deletions`, `before`, `after` |
| sync (restack) | `redundant_commits`, `before`, `after` |
| sync (clean) | `action: "cleaned"`, `reason: "merged"` |
| push | `pr_number`, `pr_url`, `created` |
| create | `branch`, `parent`, `worktree` |
| delete | `branch`, `worktree`, `dev_port`, `killed_pids`, `reparented_children` |
| rebase conflict | `action: "conflict"`, `branch`, `parent`, `conflicting_files`, `git_stderr`, `next_command` |

Commit and push receipts also include scope fields when relevant:
- `scope_defined`
- `scope_mode`
- `out_of_scope_count`
- `out_of_scope_files`

Parse with: `echo "$OUTPUT" | grep '^{' | tail -1`
