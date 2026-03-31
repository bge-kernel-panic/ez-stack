# Plan: Scope-Aware Stacking for ez-stack

## Context

`ez` already solves the mechanical side of stacked work well:
- branch parentage lives in `.git/ez/stack.json`
- worktrees isolate parallel agents
- mutation receipts let agents verify what happened
- `commit` / `push` / `sync` are the natural preflight and mutation boundaries

The failure mode still left open is simple and expensive: an agent stages files from multiple tasks, commits them to one branch, and contaminates the PR.

This plan treats "scope-aware stacking" as an **intent guard** problem, not a branch-routing or ownership system.

## Product Thesis

The first version should be **Scope Guard**:

- a branch can declare the paths it is intended to touch
- `ez` checks the staged file set before mutation
- drift outside scope is surfaced clearly to humans and agents
- strict enforcement is opt-in

This is the narrowest wedge that solves the actual problem without turning `ez` into a second VCS.

## Goals

1. Let a managed branch store an optional scope definition
2. Run scope checks before `ez commit` and `ez push -am`
3. Default to `warn`, with `strict` as an opt-in mode
4. Surface configured scope in inspection output, and scope drift in mutation-time warnings and receipts
5. Keep git as the source of truth for file matching and staged-file inspection

## Non-Goals

- auto-moving files between branches
- auto-creating sibling branches from staged files
- semantic scope inference from commit messages or diffs
- repo-wide ownership enforcement
- blocking by default
- stack-wide scope inheritance in v1

## User Experience

### Branch creation

```bash
ez create feat/auth --scope 'src/auth/**' --scope 'tests/auth/**'
ez create feat/auth --scope-mode strict --scope 'src/auth/**'
```

Behavior:
- `--scope` is repeatable
- `--scope-mode` defaults to `warn`
- no scope means current behavior, zero checks

### Commit preflight

When staged files are fully in scope:
- proceed normally

When staged files drift outside scope in `warn` mode:
- print a warning before commit
- show out-of-scope files
- suggest a fix:
  - commit specific paths with `ez commit -m "..." -- <paths>`
  - or update scope explicitly later
- still proceed

When drift occurs in `strict` mode:
- fail with exit code `5`
- print the same file list and remediation hint

### Push preflight

For `ez push -am "msg"`:
- run the same scope check after staging and before committing

For plain `ez push` with no new commit:
- no scope check in v1
- scope is guarding mutation, not PR metadata updates

### Scope management

```bash
ez scope show
ez scope add 'tests/auth/**'
ez scope set 'src/auth/**' 'tests/auth/**'
ez scope clear
```

Behavior:
- `show` prints the current branch's configured scope and mode
- `add` appends one or more patterns to the current branch
- `set` replaces the full scope on the current branch
- `clear` removes scope configuration from the current branch
- these commands only operate on managed non-trunk branches

### Inspection

`ez status`:
- whether scope exists
- scope mode
- configured patterns only

`ez status --json`:
- `scope`
- `scope_mode`
- `scope_defined`

Mutation receipts on commit/push:
- `scope_defined`
- `scope_mode`
- `out_of_scope_count`
- `out_of_scope_files`

## Design Choices

### 1. Branch-local, not stack-global

Scope belongs to a branch, not the full stack.

Reason:
- easier to explain
- easier to debug
- matches how agents and humans already think about PRs

### 2. Advisory first

Default mode is `warn`.

Reason:
- lots of legitimate cross-cutting files exist: `Cargo.lock`, shared types, migrations, test helpers
- if v1 blocks too aggressively, users disable the feature and stop trusting it

### 3. Use git path semantics, not a custom matcher

Do not invent bespoke glob behavior if avoidable.

Reason:
- `ez` is explicitly a git orchestrator
- custom matching creates long-tail correctness bugs
- staged files already come from git

Implementation note:
- matching belongs in `git.rs`, not in the pure scope-reporting module
- `scope.rs` should consume staged file sets and matching results, then compute decisions and reports
- the product contract should remain "git-compatible path-style matching", not "ez invented its own glob language"

### 4. Check staged files only

Scope validation should compare against the staged set, not unstaged working tree noise.

Reason:
- commit boundaries are what matter
- this aligns with how `ez commit` already works
- avoids false positives from unrelated dirty files in the worktree

## Data Model

Extend `BranchMeta` in `src/stack.rs`:

```rust
pub struct BranchMeta {
    pub name: String,
    pub parent: String,
    pub parent_head: String,
    pub pr_number: Option<u64>,
    pub scope: Option<Vec<String>>,
    pub scope_mode: Option<ScopeMode>,
}
```

Add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeMode {
    Warn,
    Strict,
}
```

Notes:
- `Option` keeps old state files compatible
- omitted fields mean "no scope configured" and preserve existing behavior

## CLI Surface

### `src/cli.rs`

Add to `Create`:
- `--scope <pattern>` repeatable
- `--scope-mode <warn|strict>` optional, default `warn` when scope exists

Examples:

```bash
ez create feat/auth --scope 'src/auth/**'
ez create feat/auth --scope 'src/auth/**' --scope 'tests/auth/**'
ez create feat/auth --scope-mode strict --scope 'src/auth/**'
```

## Architecture

### New pure module: `src/scope.rs`

Responsibilities:
- normalize stored scope config
- evaluate precomputed file sets against branch scope
- return a structured result for command modules to render

Suggested API:

```rust
pub enum ScopeDecision {
    NoScope,
    InScope,
    OutOfScope(ScopeReport),
}

pub struct ScopeReport {
    pub mode: ScopeMode,
    pub patterns: Vec<String>,
    pub in_scope_files: Vec<String>,
    pub out_of_scope_files: Vec<String>,
}

pub fn evaluate_scope(
    patterns: &[String],
    mode: ScopeMode,
    staged_files: &[String],
    matched_files: &[String],
) -> ScopeDecision
```

Keep this module pure and heavily unit-tested.

### `src/git.rs`

Add helper(s):
- `staged_files() -> Result<Vec<String>>`
- `filter_files_by_scope(files: &[String], patterns: &[String]) -> Result<Vec<String>>`

Implementation:
- shell out to `git diff --cached --name-only`
- use git path-style matching to compute the in-scope subset

This keeps scope evaluation focused on the staged set.

### `src/cmd/create.rs`

Responsibilities:
- accept scope args
- persist `scope` and `scope_mode` into `BranchMeta`
- include scope metadata in the create receipt

### `src/cmd/commit.rs`

Responsibilities:
- after staging logic and before mutation, fetch staged files
- delegate preflight to a shared mutation guard path
- include scope result in receipt

### `src/cmd/push.rs`

Responsibilities:
- when using `-a` / `-m`, use the same shared mutation guard path as `ez commit`
- include scope result in push receipt when a new commit was made in this invocation

### Shared mutation guard

Add a shared helper, likely in a new `src/cmd/mutation_guard.rs` or equivalent, to own:
- staged-file discovery
- scope evaluation
- warning vs strict failure behavior
- reusable receipt payload fields

Reason:
- `ez commit` and `ez push -am` currently have divergent commit flows
- scope logic will drift immediately if added twice
- the first implementation should unify the pre-commit guard path before adding more behavior

### `src/cmd/status.rs`

Responsibilities:
- show whether branch has scope configured
- include scope metadata in `--json`
- do not report dynamic drift in v1

### `src/cmd/scope.rs`

Responsibilities:
- implement `ez scope add|set|clear|show`
- mutate only the current branch's scope metadata
- validate branch context and reject trunk/unmanaged branches

## Output Design

### Human warning

Example in `warn` mode:

```text
⚠ Branch scope mismatch for `feat/auth`
  Out of scope:
    src/billing/invoice.rs
    src/shared/currency.rs
  → Commit only intended files with: ez commit -m "..." -- <paths>
  → Or update the branch scope if this branch now owns those files
```

### Strict failure

Example:

```text
✗ staged files are outside the scope for `feat/auth`
  Out of scope:
    src/billing/invoice.rs
  → Commit only intended files with: ez commit -m "..." -- <paths>
  → Or recreate/update the branch scope to match the work
```

### Receipt additions

On commit/push:

```json
{
  "cmd": "commit",
  "branch": "feat/auth",
  "scope_defined": true,
  "scope_mode": "warn",
  "out_of_scope_count": 2,
  "out_of_scope_files": ["src/billing/invoice.rs", "src/shared/currency.rs"]
}
```

If no scope is configured:

```json
{
  "scope_defined": false
}
```

## Phased Implementation

### Phase 1: Data model and creation flow

Files:
- `src/cli.rs`
- `src/stack.rs`
- `src/cmd/create.rs`

Tasks:
1. Add `ScopeMode`
2. Add optional `scope` + `scope_mode` to `BranchMeta`
3. Extend `ez create` flags
4. Persist scope metadata on branch creation
5. Add scope fields to create receipts

### Phase 2: Pure scope evaluator

Files:
- create `src/scope.rs`
- `src/main.rs` or module registration as needed

Tasks:
1. Implement scope evaluation over staged and matched file lists
2. Define stable result types
3. Add thorough unit tests for matching and report generation

### Phase 3: Scope command surface

Files:
- `src/cli.rs`
- create `src/cmd/scope.rs`
- `src/main.rs`
- `src/stack.rs`

Tasks:
1. Add `ez scope add|set|clear|show`
2. Validate managed-branch-only behavior
3. Persist mutations safely
4. Add receipts for scope mutation commands if desired, or explicitly defer them

### Phase 4: Shared commit/push guard

Files:
- `src/git.rs`
- `src/cmd/commit.rs`
- `src/cmd/push.rs`
- create `src/cmd/mutation_guard.rs` or equivalent

Tasks:
1. Add `git::staged_files()`
2. Add git-backed scope filtering helper
3. Refactor commit and `push -am` to share one pre-commit guard path
4. Run scope preflight before mutation
5. Warn vs fail based on mode
6. Add receipt fields

### Phase 5: Inspection and docs

Files:
- `src/cmd/status.rs`
- `README.md`
- `SKILL.md`
- `CLAUDE.md`

Tasks:
1. Expose scope fields in `status --json`
2. Document the feature as "Scope Guard"
3. Add examples for agents:
   - narrow branch scope
   - strict mode
   - path-scoped commit remediation
   - `ez scope add|set|clear|show`
4. Keep `status` config-only in v1, mutation-time checks only
5. Move scope-aware stacking from "deferred" to shipped roadmap once implemented

## Testing

### Unit tests

Add pure tests for:
- no scope configured
- one pattern, one in-scope file
- mixed in-scope and out-of-scope files
- multiple patterns
- empty staged file list
- strict vs warn report shape

Add command tests for:
- `scope add` appends patterns
- `scope set` replaces patterns
- `scope clear` removes configuration
- `scope show` renders empty vs configured scope correctly

Add serialization tests for:
- old `stack.json` loading without scope fields
- new `stack.json` round-trip with scope fields

### Command-level verification

Manual verification:

```bash
ez create feat/auth --scope 'src/auth/**'
touch src/auth/a.rs src/billing/b.rs
git add src/auth/a.rs src/billing/b.rs
ez commit -m "test scope"
```

Expected:
- warning in `warn` mode
- failure in `strict` mode

### Regression tests

Must confirm:
- branches without scope are unchanged
- worktree creation still works
- receipts remain valid JSON
- `cargo test`
- `cargo clippy -- -D warnings`
- `cargo fmt --all -- --check`

## Risks

### Risk 1: False positives on shared/generated files

Mitigation:
- keep v1 in `warn` mode by default
- document that strict mode is for disciplined repos only

### Risk 2: Matching semantics surprise users

Mitigation:
- keep accepted pattern language small and explicit
- document examples in `--help` and README
- avoid "smart" inference in v1

### Risk 3: Too much output noise for agents

Mitigation:
- only emit detailed warnings when drift exists
- keep receipts structured and compact

## Success Criteria

This feature is successful if:

1. An agent working on `feat/auth` gets a clear warning before committing `src/billing/*`
2. A human can inspect and edit branch intent with `ez scope show|add|set|clear`
3. Existing repos without scope configuration behave exactly as before
4. `ez commit` and `ez push -am` use one shared scope-guard pipeline
5. The feature makes accidental PR contamination rarer without forcing users into constant override flows

## Recommendation

Ship this as **Scope Guard** in v1.

That name reflects what the feature actually does. It protects branch intent at mutation time. It does not promise magic routing, ownership, or automatic branch management.

That is the right first version.
