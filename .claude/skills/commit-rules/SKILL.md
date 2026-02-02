---
name: commit-rules
description: Commit message format (gitmoji), PR template, and commit granularity rules for the Gantry Board project. Reference this when creating commits or pull requests.
user-invocable: false
---

# Commit & PR Rules

## Commit Message Format

```
<emoji> <scope>: <subject>

<body>

<footer>
```

### Emoji (required) — gitmoji

| Emoji | Meaning |
|-------|---------|
| ✨ | New feature |
| 🐛 | Bug fix |
| 📝 | Documentation only |
| ✅ | Add or update tests |
| ♻️ | Refactor (no behavior change) |
| 🔧 | Build, CI, or config changes |
| 🎨 | Code formatting only (clippy, fmt, prettier) |
| ⚡️ | Performance improvement |
| 🔥 | Remove code or files |
| 💥 | Breaking change |
| 🚀 | Deploy |
| 🚧 | Work in progress |
| 🔒 | Security fix |
| ⬆️ | Upgrade dependencies |
| 🗃️ | Database migration |

### Scope (recommended)

`backend`, `frontend`, `db`, `agent`, `git`, `docker`, `ci`

### Subject (required)

- English, lowercase start, no trailing period
- 50 characters max
- Imperative mood: "add", "fix", "remove" (not "added", "fixed")

### Body (optional)

- Explain **why**, not what (the diff shows what)
- Wrap at 72 characters

### Footer (optional)

- `BREAKING CHANGE: <description>`
- `Closes #<issue>` / `Refs #<issue>`

### Examples

```
✨ backend: add health check endpoint

The /health endpoint returns "ok" for basic liveness probing.
This will be used by docker-compose health checks.
```

```
✅ backend: add task creation tests

Write failing tests first per TDD workflow.
Tests cover create, read, update operations.

Refs #12
```

## Commit Granularity Rules

### Code Changes

- Max **10 files** per commit
- Max **300 lines** added+deleted (excluding tests and auto-generated files)
- **1 commit = 1 concern** — never mix feat and fix in the same commit
- Auto-generated files (Cargo.lock, package-lock.json) do not count toward limits
- If limits are exceeded, split the commit. If splitting is impractical, explain why in the body.

### Test Code

- **1 commit = tests for 1 feature** (one endpoint, one module, one component)
- Name the test target explicitly in the subject
- Max **5 test files** per commit
- Related unit + integration tests for the same module may be combined

### TDD Commit Pattern

Always separate test commits from implementation commits:

```
✅ backend: add task creation tests        ← tests only (expected to fail)
✨ backend: implement task creation         ← implementation (tests pass)
♻️ backend: extract task validation logic   ← refactor (tests still pass)
```

- TDD test commits MUST be made before the implementation commit
- Test commits are in a failing state — note this in the body if needed
- Never modify tests during the implementation phase

## Pull Request Rules

- Title: English, max 70 characters
- Summary section must not be empty
- All PRs must reference related issues
- Test Plan must include verification steps

## Language

All commit messages and PR descriptions must be in **English**.
