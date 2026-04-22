---
name: rust-commit-flow
description: >
  Rust project commit workflow with Conventional Commits and cargo-based pre-commit quality gates.
  Use when committing Rust code changes, staging files, writing commit messages, or when the user
  says "commit", "git commit", "stage and commit", "pre-commit check", "quality gate", "lint before commit",
  or any phrase involving the git commit process in a Rust project. Also trigger when the user asks
  to check code quality before committing, or when they want help crafting a commit message.
---

# Rust Commit Flow

A structured commit workflow for Rust projects that enforces code quality through cargo-based pre-commit
checks and generates well-formed Conventional Commit messages derived from actual project history patterns.

## Why This Skill Exists

Committing code without running quality gates leads to broken builds, CI failures, and messy history.
This skill automates the discipline: run checks first, then craft a commit message that matches the
project's existing conventions. Every step uses the Rust toolchain exclusively — no external linters,
no Node-based tools, no Python formatters.

## The Commit Workflow

Follow this exact sequence when the user wants to commit. Do not skip steps. Do not parallelize
cargo commands (Cargo holds a file lock on `target/`).

### Phase 1: Analyze Changes

Before any quality gate, understand what changed so you can write an accurate commit message later.

```bash
# Get the full diff summary
git diff --stat

# Get the actual diff content for staged changes
git diff --cached --stat
git diff --cached

# If nothing is staged yet, check unstaged changes
git diff --stat
git status --short
```

Read the diff output carefully. Note:
- Which files changed and in which crates (message/, server/, frontend/, css-processor/)
- What kind of change: new feature, bug fix, refactoring, performance tweak, dependency update, test addition
- The scope of the change — does it affect one module or the whole workspace?

### Phase 2: Stage Files

Stage only the files relevant to this commit. Avoid `git add .` unless the user explicitly wants it.

```bash
# Stage specific files
git add path/to/file.rs path/to/another_file.rs

# Or stage by pattern if appropriate
git add server/src/
```

Verify what's staged:
```bash
git diff --cached --stat
```

### Phase 3: Pre-Commit Quality Gates (MUST pass before committing)

These checks run sequentially because Cargo uses file locks. Use the background execution
pattern from the project's Rust shell output rules.

> **Important**: Only run these checks if `.rs` files are in the staged changes. If only
> non-Rust files changed (markdown, config, CSS), skip the cargo gates and go straight to commit.

#### Gate 1: Format Check

```bash
cargo fmt --all -- --check
```

If this fails, format the code:
```bash
cargo fmt --all
```

Then re-stage the formatted files:
```bash
git add -u
```

#### Gate 2: Compile Check

```bash
# Background execution pattern (Rust compilation is slow)
rm -f /tmp/cargo-check-done /tmp/cargo-check-output.txt
(cargo check --all 2>&1 | tee /tmp/cargo-check-output.txt; echo $? > /tmp/cargo-check-done) &
while [ ! -f /tmp/cargo-check-done ]; do sleep 10; done
cat /tmp/cargo-check-done    # Must be 0
cat /tmp/cargo-check-output.txt
```

If this fails, report the errors to the user. Do not proceed to commit.

#### Gate 3: Clippy (strict — denies warnings)

```bash
rm -f /tmp/cargo-clippy-done /tmp/cargo-clippy-output.txt
(cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tee /tmp/cargo-clippy-output.txt; echo $? > /tmp/cargo-clippy-done) &
while [ ! -f /tmp/cargo-clippy-done ]; do sleep 10; done
cat /tmp/cargo-clippy-done    # Must be 0
cat /tmp/cargo-clippy-output.txt
```

If this fails, fix the clippy issues (do NOT add `#[allow(...)]` to suppress — fix the actual code),
then re-stage and re-run from Gate 1.

#### Gate 4: Unit Tests

```bash
rm -f /tmp/cargo-test-done /tmp/cargo-test-output.txt
(cargo test --lib 2>&1 | tee /tmp/cargo-test-output.txt; echo $? > /tmp/cargo-test-done) &
while [ ! -f /tmp/cargo-test-done ]; do sleep 10; done
cat /tmp/cargo-test-done    # Must be 0
cat /tmp/cargo-test-output.txt
```

If this fails, report the failures. Do not proceed to commit.

**Gate Failure Protocol**: If any gate fails, stop and inform the user. Present the error output
clearly. Suggest fixes. Do not attempt to commit until all gates pass.

### Phase 4: Generate Commit Message

Based on the analysis from Phase 1 and the project's Conventional Commits convention, craft
the commit message.

#### Commit Message Format

```
<type>: <lowercase description without trailing period>
```

#### Allowed Types (derived from project history)

| Type | When to Use | Example |
|------|-------------|---------|
| `feat` | New feature or significant functionality addition | `feat: add WebSocket reconnection with exponential backoff` |
| `fix` | Bug fix | `fix: resolve UUID collision in session management` |
| `perf` | Performance improvement | `perf: optimize scroll behavior of message list` |
| `refactor` | Code restructuring without behavior change | `refactor: restructure the RTC connection procedure` |
| `chore` | Maintenance, dependency updates, config changes | `chore: update tokio to 1.52.0` |
| `test` | Adding or updating tests | `test: add integration tests for message serialization` |
| `docs` | Documentation only changes | `docs: add API documentation for auth module` |
| `style` | Formatting, whitespace (non-functional) | `style: fix trailing whitespace in server module` |
| `ci` | CI/CD configuration changes | `ci: add coverage report to CI pipeline` |
| `build` | Build system or dependency changes | `build: upgrade to edition 2024` |

#### Message Construction Rules

1. **Type selection**: Match the type to the primary intent of the change, not a secondary effect.
   If a change both adds a feature AND fixes a bug, use `feat` if the feature is the primary goal.

2. **Description**: Write in imperative mood ("add", not "added" or "adds"). Lowercase. No period.
   Keep it under 72 characters total (type + colon + space + description).

3. **Scope** (optional): If the change is clearly scoped to one crate or module, add it in parens:
   ```
   feat(server): add rate limiting middleware
   fix(message): resolve deserialization error for empty payload
   ```

4. **Body** (optional): If the change is complex, add a blank line then a body explaining WHY,
   not WHAT (the diff shows what). Wrap at 72 characters.

5. **English only**: All commit messages must be in English, following the project's established
   convention from git history. No Chinese characters.

6. **Spelling**: Double-check spelling. The project history contains typos like "transimission",
   "cononent", "connnect" — do not repeat these.

#### Message Examples

```
feat: add JWT token refresh endpoint
fix: resolve race condition in WebSocket broadcast
perf: reduce allocations in message deserialization
refactor: extract auth logic into separate module
chore: update cargo-make to 0.37.0
test: add proptest for message protocol encoding
feat(frontend): add user avatar upload component
fix(server): handle missing Content-Type header gracefully
```

### Phase 5: Execute Commit

```bash
git commit -m "<type>: <description>"
```

Or with a body:
```bash
git commit -m "<type>: <description>

<detailed explanation>"
```

### Phase 6: Post-Commit Verification

After committing, verify the commit was created correctly:

```bash
git log -1 --stat
```

Confirm:
- The commit hash was generated
- The message matches what was intended
- The file list matches what was staged

## Quick Commit (Skip Gates)

If the user explicitly wants to skip quality gates (e.g., for a documentation-only change or WIP),
still follow the commit message format but skip Phase 3. Warn the user:

> ⚠️ Skipping pre-commit quality gates. This commit has not been validated against cargo check/clippy/test.

## Commit Message from Diff Analysis

When the user asks for help writing a commit message without going through the full flow,
analyze the staged diff and produce a message following the rules above. Present 2-3 options
ranked by accuracy so the user can choose.

## Handling Special Cases

### Merge Conflicts in Cargo.lock

After resolving merge conflicts, always run `cargo check` to regenerate the lock file correctly
before committing.

### Large Changesets

If the diff touches more than 10 files across multiple crates, suggest splitting into smaller
commits organized by concern. Each commit should pass all quality gates independently.

### WIP Commits

For work-in-progress commits, prefix with `wip:` instead of a conventional type:
```
wip: implementing E2EE key exchange
```
This signals that quality gates are intentionally skipped and the code is not ready for review.

### Amending Commits

When amending the last commit:
1. Re-run quality gates if `.rs` files changed
2. Use `git commit --amend` with the updated message
3. Never amend commits that have already been pushed to a shared branch
