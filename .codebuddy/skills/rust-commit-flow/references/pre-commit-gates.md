# Pre-Commit Quality Gates Reference

## Overview

This project uses a strict four-gate pre-commit pipeline. Every gate must pass before
a commit is allowed. All gates use the Rust toolchain exclusively.

## Gate 1: Format Check

**Command**: `cargo fmt --all -- --check`

**What it catches**: Inconsistent formatting, wrong indentation, missing line breaks,
misaligned struct fields.

**Auto-fix**: `cargo fmt --all`

**Project config** (from `rustfmt.toml`):
- max_width = 100
- tab_spaces = 2
- edition = "2024"
- reorder_imports = true
- reorder_modules = true

**After auto-fix**: Re-stage changed files with `git add -u`

## Gate 2: Compile Check

**Command**: `cargo check --all`

**What it catches**: Type errors, missing imports, dead code, wrong function signatures,
trait bound failures, mismatched types.

**No auto-fix**: Must manually correct all compilation errors.

**Project config** (from `Cargo.toml` workspace):
- edition = "2024"
- rust-version = "1.88"
- Workspace members: message, server, frontend, css-processor

## Gate 3: Clippy

**Command**: `cargo clippy --all-targets --all-features -- -D warnings`

**What it catches**: Code smells, unnecessary allocations, redundant clones, missing docs,
incorrect error handling, style violations, complexity issues.

**No auto-fix**: Must manually correct all clippy warnings.

**Important rules**:
- Do NOT add `#[allow(...)]` attributes to suppress clippy warnings
- Fix the actual code issue instead
- The project uses `clippy::pedantic = deny` and `clippy::all = deny`

**Project lint config** (from `Cargo.toml`):
- clippy pedantic = deny
- clippy all = deny
- Allowed exceptions: missing_errors_doc, missing_panics_doc, module_name_repetitions,
  must_use_candidate, similar_names, too_many_lines, unused_self, single_match_else,
  enum_glob_use, cast_possible_truncation, cast_sign_loss, cast_precision_loss,
  cast_lossless, multiple_crate_versions, significant_drop_tightening

## Gate 4: Unit Tests

**Command**: `cargo test --lib`

**What it catches**: Regressions, broken invariants, incorrect logic, serialization failures.

**No auto-fix**: Must manually correct failing tests.

**Test organization**:
- Unit tests: `#[cfg(test)] mod tests` within each source file
- Integration tests: `tests/` directory (not part of pre-commit gate)
- WASM tests: separate `test-wasm` task (not part of pre-commit gate)

## Execution Rules

### Sequential Execution Required

Cargo acquires a file lock on `target/` during compilation. Running multiple cargo
commands simultaneously causes lock conflicts and failures.

**Correct order**: fmt → check → clippy → test

### Background Execution Pattern

Rust compilation is slow. Use this pattern to avoid shell timeouts:

```bash
# Clean previous run artifacts
rm -f /tmp/cargo-<gate>-done /tmp/cargo-<gate>-output.txt

# Run in background, capture output and exit code
(cargo <command> 2>&1 | tee /tmp/cargo-<gate>-output.txt; echo $? > /tmp/cargo-<gate>-done) &

# Poll for completion
while [ ! -f /tmp/cargo-<gate>-done ]; do sleep 10; done

# Check result
EXIT_CODE=$(cat /tmp/cargo-<gate>-done)
if [ "$EXIT_CODE" != "0" ]; then
  echo "Gate failed with exit code $EXIT_CODE"
  cat /tmp/cargo-<gate>-output.txt
  exit 1
fi
```

### Complete Pipeline Script

```bash
#!/bin/bash
set -e

echo "=== Gate 1: Format Check ==="
cargo fmt --all -- --check
echo "✓ Format check passed"

echo "=== Gate 2: Compile Check ==="
rm -f /tmp/cargo-check-done /tmp/cargo-check-output.txt
(cargo check --all 2>&1 | tee /tmp/cargo-check-output.txt; echo $? > /tmp/cargo-check-done) &
while [ ! -f /tmp/cargo-check-done ]; do sleep 10; done
CHECK_EXIT=$(cat /tmp/cargo-check-done)
if [ "$CHECK_EXIT" != "0" ]; then
  echo "✗ Compile check failed"
  cat /tmp/cargo-check-output.txt
  exit 1
fi
echo "✓ Compile check passed"

echo "=== Gate 3: Clippy ==="
rm -f /tmp/cargo-clippy-done /tmp/cargo-clippy-output.txt
(cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tee /tmp/cargo-clippy-output.txt; echo $? > /tmp/cargo-clippy-done) &
while [ ! -f /tmp/cargo-clippy-done ]; do sleep 10; done
CLIPPY_EXIT=$(cat /tmp/cargo-clippy-done)
if [ "$CLIPPY_EXIT" != "0" ]; then
  echo "✗ Clippy check failed"
  cat /tmp/cargo-clippy-output.txt
  exit 1
fi
echo "✓ Clippy check passed"

echo "=== Gate 4: Unit Tests ==="
rm -f /tmp/cargo-test-done /tmp/cargo-test-output.txt
(cargo test --lib 2>&1 | tee /tmp/cargo-test-output.txt; echo $? > /tmp/cargo-test-done) &
while [ ! -f /tmp/cargo-test-done ]; do sleep 10; done
TEST_EXIT=$(cat /tmp/cargo-test-done)
if [ "$TEST_EXIT" != "0" ]; then
  echo "✗ Unit tests failed"
  cat /tmp/cargo-test-output.txt
  exit 1
fi
echo "✓ Unit tests passed"

echo ""
echo "✅ All quality gates passed. Ready to commit."
```

## Failure Handling Protocol

When a gate fails:

1. **Format**: Auto-fix and re-stage. Then restart from Gate 1.
2. **Check**: Report errors. User must fix. Re-run from Gate 1 after fixes.
3. **Clippy**: Report warnings. User must fix (no `#[allow]` suppression). Re-run from Gate 1.
4. **Test**: Report failures. User must fix. Re-run from Gate 1 after fixes.

**Never proceed to commit if any gate fails.**
