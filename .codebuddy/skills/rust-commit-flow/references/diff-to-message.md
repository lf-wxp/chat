# Diff-to-Commit-Message Mapping Rules

## How to Read a Diff and Choose the Right Type

Analyzing the staged diff is the first step in crafting an accurate commit message.
This document provides systematic rules for mapping code changes to commit types.

## Decision Tree

```
START: What is the primary intent of this change?
│
├─ Does it add new user-facing or API functionality?
│  └─ YES → feat
│
├─ Does it fix a bug, error, or incorrect behavior?
│  └─ YES → fix
│
├─ Does it improve speed or reduce resource usage?
│  └─ YES → perf
│
├─ Does it restructure code without changing behavior?
│  └─ YES → refactor
│
├─ Does it add or update tests?
│  └─ YES → test
│
├─ Does it change documentation or comments only?
│  └─ YES → docs
│
├─ Does it fix formatting or whitespace only?
│  └─ YES → style
│
├─ Does it change CI/CD configuration?
│  └─ YES → ci
│
├─ Does it change build config or dependencies?
│  └─ YES → build
│
└─ Is it maintenance with no functional impact?
   └─ YES → chore
```

## File Pattern Heuristics

| File Pattern | Likely Type | Notes |
|-------------|-------------|-------|
| `**/mod.rs` (new module) | `feat` | New module = new feature area |
| `**/test*.rs` or `#[test]` blocks | `test` | Test-only changes |
| `Cargo.toml` (dependency version change) | `chore` or `build` | `build` if changing build config, `chore` for dep bumps |
| `**/*.md` only | `docs` | Documentation only |
| `.github/workflows/*` | `ci` | CI configuration |
| `rustfmt.toml`, `clippy.toml` | `chore` | Tool configuration |
| `**/error.rs` | `fix` or `feat` | Error handling: fix if correcting, feat if adding new types |
| `**/config.rs` | `chore` or `feat` | Config changes: chore if tuning, feat if new options |
| Renaming files/moving code | `refactor` | Structural change without behavior change |
| Adding `unsafe` blocks | `feat` or `perf` | Usually for FFI or performance, note in body |

## Description Writing Rules

### Imperative Mood

The description must use imperative mood — as if giving a command:

- ✅ `add WebSocket reconnection logic`
- ❌ `added WebSocket reconnection logic`
- ❌ `adds WebSocket reconnection logic`
- ❌ `adding WebSocket reconnection logic`

### Common Verb Patterns by Type

| Type | Common Verbs |
|------|-------------|
| `feat` | add, implement, introduce, support, create, enable |
| `fix` | resolve, handle, correct, prevent, repair, fix |
| `perf` | optimize, reduce, improve, cache, avoid, eliminate |
| `refactor` | extract, move, restructure, rename, consolidate, simplify |
| `test` | add, update, increase, expand, cover |
| `docs` | add, update, clarify, document |
| `chore` | update, bump, upgrade, remove, clean |
| `style` | fix, normalize, align |
| `ci` | add, update, configure, enable |
| `build` | upgrade, update, migrate, configure |

### Length Guidelines

- Target: 50 characters for the description part (type + `: ` + description ≈ 72 chars total)
- Hard limit: 72 characters total for the first line
- If you can't fit the description in ~50 chars, use a scope or simplify

### Scope Usage

Add a scope when the change is clearly contained within one workspace crate:

| Scope | When to Use |
|-------|-------------|
| `message` | Changes only in the `message/` crate |
| `server` | Changes only in the `server/` crate |
| `frontend` | Changes only in the `frontend/` crate |
| `css-processor` | Changes only in the `css-processor/` crate |
| `workspace` | Changes affecting Cargo.toml workspace root, Makefile.toml, or cross-crate config |

**Do not use scope** when changes span multiple crates. Omit it rather than choosing
an inaccurate scope.

## Multi-Concern Changesets

When a single diff contains multiple types of changes:

1. **Prefer splitting**: If possible, suggest the user split into separate commits.
   Each commit should express one clear intent.

2. **Choose the dominant type**: If splitting isn't practical, use the type that
   best represents the primary purpose of the change.

3. **Body explanation**: Use the commit body to mention secondary changes:
   ```
   feat: add user presence tracking

   Also includes minor refactoring of the connection manager
   to support the new presence event handler.
   ```

## Examples by Type

### feat
```
feat: add E2EE key exchange protocol
feat(server): add rate limiting middleware
feat(frontend): implement message search with debouncing
feat(message): support protocol v2 deserialization
```

### fix
```
fix: resolve UUID collision in session tokens
fix(server): handle abrupt WebSocket disconnect gracefully
fix(frontend): correct scroll position after new message
```

### perf
```
perf: reduce allocations in message deserialization
perf(frontend): optimize re-render on message list update
perf(server): replace mutex with RwLock for read-heavy state
```

### refactor
```
refactor: extract auth logic into standalone module
refactor(server): consolidate connection state management
refactor(frontend): replace callback props with context signals
```

### chore
```
chore: update tokio to 1.52.0
chore: remove deprecated derive macro
chore: clean up unused dev-dependencies
```

### test
```
test: add proptest for message protocol encoding
test(server): add integration test for auth flow
```
