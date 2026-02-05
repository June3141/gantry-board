---
name: quality-checks
description: Taskfile commands for code quality checks and formatting
---

# Quality Checks

## Commands

| Check | Command | When |
|-------|---------|------|
| Format (fix) | `task fmt` | After writing code |
| Format (check) | `task fmt:check` | CI / verify only |
| Lint | `task lint` | After completing a feature |
| Build | `task build` | Before commit |
| Test | `task test` | After implementation passes lint |
| Full check | `task check` | Before commit (auto by L3 hook) |
| API export | `task api:export` | After modifying utoipa annotations |
| API generate | `task api:generate` | After OpenAPI spec changes |
| API diff | `task api:diff` | CI — verify generated code is up to date |

## Layer-specific Commands

| Layer | Format | Lint | Build | Test |
|-------|--------|------|-------|------|
| Backend | `task backend:fmt` | `task backend:lint` | `task backend:build` | `task backend:test` |
| Frontend | `task frontend:fmt` | `task frontend:lint` | `task frontend:build` | `task frontend:test` |

## Automated Hooks

Quality is enforced automatically via 3 layers of Claude Code hooks:

- **L1 (PostToolUse)**: Auto-formats files after every Write/Edit (`cargo fmt` / `biome format --write`)
- **L2 (Stop)**: Runs lint on modified layers when Claude completes a response
- **L3 (PreToolUse)**: Blocks `git commit` unless commit message format, commit size, and `task check` all pass
