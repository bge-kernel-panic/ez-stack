# rs

**Stacked PRs for GitHub.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/rohoswagger/rs-stack/actions/workflows/ci.yml/badge.svg)](https://github.com/rohoswagger/rs-stack/actions/workflows/ci.yml)

---

`rs` is a fast, lightweight CLI for managing stacked pull requests on GitHub. It shells out to `git` and `gh` so there's nothing magical happening under the hood — just the tools you already know, orchestrated intelligently.

## Why stacked PRs?

Large pull requests are hard to review. Stacked PRs let you break work into a chain of small, focused branches where each branch builds on the one below it:

```
main
 └── feat/auth-types        ← PR #1 (data models)
      └── feat/auth-api     ← PR #2 (API routes, depends on #1)
           └── feat/auth-ui ← PR #3 (frontend, depends on #2)
```

Reviewers see small diffs. You keep working without waiting. When PR #1 merges, `rs` rebases the rest of the stack automatically.

The problem is that `git` doesn't know about stacks. Rebasing, reordering, and keeping GitHub PRs pointed at the right base branch is tedious and error-prone. `rs` handles all of that for you.

## Quick start

```bash
# Install
cargo install rs-stack

# Initialize in any git repo
cd your-repo
rs init

# Start building a stack
rs create feat/parse-config
# ... make changes ...
rs commit -m "add config parser"

rs create feat/use-config
# ... make changes ...
rs commit -m "wire config into app"

# Push and open PRs for the whole stack
rs push
rs submit
```

That's it. Two PRs, correctly chained, with GitHub base branches set automatically.

## Commands

### Stack creation & editing

| Command | Description |
|---------|-------------|
| `rs init` | Initialize `rs` in the current repository |
| `rs create <name>` | Create a new branch on top of the current stack |
| `rs commit [-m <msg>]` | Commit staged changes to the current branch |
| `rs amend` | Amend the last commit on the current branch |
| `rs delete [<name>]` | Delete a branch from the stack and restack |
| `rs move <--up\|--down>` | Reorder the current branch within the stack |

### Syncing & rebasing

| Command | Description |
|---------|-------------|
| `rs sync` | Fetch `main`, rebase the entire stack, clean up merged branches |
| `rs restack` | Rebase each branch in the stack onto its parent |
| `rs push` | Force-push all branches in the stack to the remote |

### Navigation

| Command | Description |
|---------|-------------|
| `rs up` | Check out the branch above the current one |
| `rs down` | Check out the branch below the current one |
| `rs top` | Check out the top of the stack |
| `rs bottom` | Check out the bottom of the stack |
| `rs checkout <name>` | Check out any branch in the stack by name |

### GitHub integration

| Command | Description |
|---------|-------------|
| `rs submit` | Create or update GitHub PRs for all branches in the stack |
| `rs merge` | Merge the bottom PR and restack |

### Inspection

| Command | Description |
|---------|-------------|
| `rs log` | Show the full stack with branch names, commit counts, and PR status |
| `rs status` | Show the current branch and its position in the stack |

## Example workflow

Here's a complete session building a three-branch stack:

```bash
# 1. Start from main
git checkout main && git pull
rs init

# 2. Create the first branch in the stack
rs create feat/auth-types
cat > src/auth/types.rs << 'EOF'
pub struct User { pub id: u64, pub email: String }
pub struct Session { pub token: String, pub user_id: u64 }
EOF
rs commit -m "define User and Session types"

# 3. Stack a second branch on top
rs create feat/auth-api
cat > src/auth/api.rs << 'EOF'
pub fn login(email: &str) -> Session { /* ... */ }
pub fn logout(session: &Session) { /* ... */ }
EOF
rs commit -m "add login/logout API"

# 4. Stack a third branch on top
rs create feat/auth-middleware
cat > src/middleware/auth.rs << 'EOF'
pub fn require_auth(req: &Request) -> Result<User, AuthError> { /* ... */ }
EOF
rs commit -m "add auth middleware"

# 5. See the full stack
rs log
#   main
#   ├── feat/auth-types        (1 commit)
#   │   ├── feat/auth-api      (1 commit)
#   │   │   ├── feat/auth-middleware (1 commit)  ← you are here

# 6. Push everything and open PRs
rs push
rs submit
# Creates 3 PRs:
#   feat/auth-types        → main
#   feat/auth-api          → feat/auth-types
#   feat/auth-middleware    → feat/auth-api

# 7. After feat/auth-types is reviewed and merged on GitHub:
rs sync
# Fetches main (which now includes auth-types),
# rebases auth-api onto main, rebases auth-middleware onto auth-api,
# deletes the merged feat/auth-types branch,
# and updates PR base branches on GitHub.
```

## How it works

`rs` is intentionally simple in its architecture:

- **No custom git internals.** Every git operation is a call to the `git` CLI. Every GitHub operation goes through `gh`. You can always see exactly what happened by reading your git log.
- **Stack metadata** is stored in `.git/rs/stack.json` — a single JSON file tracking branch order, parent relationships, and associated PR numbers. It's local to your repo and ignored by git.
- **Restacking** uses `git rebase --onto` to move each branch in the stack onto its updated parent. This is the same operation you'd do by hand; `rs` just does it for every branch in the right order.
- **PR management** calls `gh pr create` and `gh pr edit` to set base branches so GitHub shows the correct, minimal diff for each PR in the stack.

### Stack metadata format

```json
{
  "version": 1,
  "trunk": "main",
  "branches": [
    { "name": "feat/auth-types", "parent": "main", "pr": 101 },
    { "name": "feat/auth-api", "parent": "feat/auth-types", "pr": 102 },
    { "name": "feat/auth-middleware", "parent": "feat/auth-api", "pr": null }
  ]
}
```

## Prerequisites

- **git** 2.38+
- **gh** (GitHub CLI), authenticated via `gh auth login`
- A GitHub repository with push access

## Installation

### From crates.io

```bash
cargo install rs-stack
```

### From source

```bash
git clone https://github.com/rohoswagger/rs-stack.git
cd rs
cargo install --path .
```

### Install script (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/rohoswagger/rs-stack/main/install.sh | bash
```

To install a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/rohoswagger/rs-stack/main/install.sh | bash -s -- v0.1.0
```

### GitHub releases

Pre-built binaries for Linux and macOS are available on the [Releases](https://github.com/rohoswagger/rs-stack/releases) page.

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style, and how to submit changes.

## License

MIT. See [LICENSE](LICENSE) for details.
