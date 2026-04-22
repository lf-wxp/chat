# Conventional Commits Reference for This Project

## Derived from Git History Analysis

Analysis of 200 commits in this repository revealed the following patterns.

## Type Distribution

| Type | Count | Percentage | Description |
|------|-------|-----------|-------------|
| `feat` | 108 | 86.4% | New feature or functionality |
| `perf` | 6 | 4.8% | Performance improvement |
| `refactor` | 4 | 3.2% | Code restructuring, no behavior change |
| `chore` | 4 | 3.2% | Maintenance, deps, config |
| `fix` | 2 | 1.6% | Bug fix |
| `test` | 0 | 0% | Test additions (recommended, not yet used) |
| `docs` | 0 | 0% | Documentation changes (recommended, not yet used) |
| `style` | 0 | 0% | Formatting only (recommended, not yet used) |
| `ci` | 0 | 0% | CI/CD changes (recommended, not yet used) |
| `build` | 0 | 0% | Build system changes (recommended, not yet used) |

## Message Format

```
<type>: <lowercase imperative description>
```

### Rules Extracted from History

1. Type and description separated by `: ` (colon + space)
2. Description is always lowercase
3. No trailing period in description
4. No body or footer in existing commits
5. Imperative mood ("add" not "added")
6. English only — no Chinese characters

### Known Typos to Avoid

These typos appeared in the commit history and must not be repeated:

- ❌ `transimission` → ✅ `transmission`
- ❌ `comonent` → ✅ `component`
- ❌ `connnect` → ✅ `connect`
- ❌ `recieve` → ✅ `receive`
- ❌ `occured` → ✅ `occurred`

## Scope Patterns

The project has a Cargo workspace with these crates:

- `message` — message protocol and serialization
- `server` — backend server logic
- `frontend` — Leptos-based WASM frontend
- `css-processor` — CSS processing utilities

Use scope when a change is clearly contained in one crate:

```
feat(message): add protobuf serialization support
fix(server): resolve connection pool exhaustion
refactor(frontend): extract signal logic from view components
```

## Quality Gate Requirements

Pre-commit checks use only Rust toolchain tools:

| Gate | Command | Purpose | Blocking |
|------|---------|---------|----------|
| Format | `cargo fmt --all -- --check` | Code formatting | Yes |
| Compile | `cargo check --all` | Type checking & compilation | Yes |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` | Code quality | Yes |
| Test | `cargo test --lib` | Unit test execution | Yes |

### When to Skip Gates

- Non-Rust file changes only (`.md`, `.toml` comments, `.css`)
- WIP commits (must use `wip:` prefix)
- User explicitly requests skip (must warn about risks)

### Execution Pattern

Cargo commands must run sequentially due to file locks. Use the background execution
pattern with marker files as specified in the project's Rust shell output rules.
