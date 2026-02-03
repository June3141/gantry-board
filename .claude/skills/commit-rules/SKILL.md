---
name: commit-rules
description: Commit message format (gitmoji), PR template, and commit granularity rules for the Gantry Board project. Reference this when creating commits or pull requests.
user-invocable: false
---

# Commit & PR Rules

## Commit Message Format

```
<type>: <emoji> <subject>

<body>

<footer>
```

### Type (required)

| Type | When to use |
|------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `test` | Add or update tests |
| `refactor` | Refactor (no behavior change) |
| `chore` | Build, CI, or config changes |
| `style` | Code formatting only (clippy, fmt, prettier) |
| `perf` | Performance improvement |

### Emoji (required) — gitmoji

Place after the colon. Full list is generated from upstream and stored in `.claude/hooks/gitmoji-pattern.txt`.
Commonly used:

| Emoji | Meaning |
|-------|---------|
| ✨ | New feature |
| 🐛 | Bug fix |
| 📝 | Documentation only |
| ✅ | Add or update tests |
| ♻️ | Refactor (no behavior change) |
| 🔧 | Build, CI, or config changes |
| 🎨 | Code formatting only |
| ⚡️ | Performance improvement |
| 🔥 | Remove code or files |
| 💥 | Breaking change |
| 🚀 | Deploy |
| 🚧 | Work in progress |
| 🔒 | Security fix |
| ⬆️ | Upgrade dependencies |
| 🗃️ | Database migration |
| 🎉 | Initial commit |

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
feat: ✨ add health check endpoint

The /health endpoint returns "ok" for basic liveness probing.
This will be used by docker-compose health checks.
```

```
test: ✅ add task creation tests

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
test: ✅ add task creation tests             ← tests only (expected to fail)
feat: ✨ implement task creation              ← implementation (tests pass)
refactor: ♻️ extract task validation logic   ← refactor (tests still pass)
```

- TDD test commits MUST be made before the implementation commit
- Test commits are in a failing state — note this in the body if needed
- Never modify tests during the implementation phase

## Pull Request Rules

### PR Title Format

PR タイトルはコミットメッセージと同じ `type: emoji subject` 形式を使う（squash merge 時にそのままコミットメッセージになるため）:

```
<type>: <emoji> <subject>
```

- Max 70 characters
- Same type/emoji rules as commit messages
- Examples:
  - `feat: ✨ add task CRUD endpoints`
  - `fix: 🐛 resolve board drag-and-drop race condition`
  - `chore: 🔧 add PR title validation`

### PR Body

PR テンプレート (`.github/pull_request_template.md`) のセクションをすべて埋める:

- **Summary**: 変更内容を 1-3 文で説明
- **Changes**: 主な変更点をリスト
- **Why**: なぜこの変更が必要か
- **Test Plan**: 検証手順をチェックリストで記載
- **Related**: 関連 issue を `Closes #` / `Refs #` で参照

### General

- All PRs must reference related issues when applicable
- Test Plan must include verification steps
- Summary section must not be empty

## Language

All commit messages and PR descriptions must be in **English**.
