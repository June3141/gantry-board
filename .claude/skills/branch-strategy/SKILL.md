---
name: branch-strategy
description: Branch naming conventions, merge rules, worktree patterns, and protection settings for the Gantry Board project. Reference this when creating branches, merging, or setting up worktrees.
user-invocable: false
---

# Branch Strategy

## Branch Structure

| Branch | Purpose | Base | Merge target |
|--------|---------|------|-------------|
| `main` | Released stable code | — | — |
| `develop` | Development integration | `main` | `main` (via release) |
| `feat/<task-id>-<slug>` | New feature | `develop` | `develop` |
| `fix/<task-id>-<slug>` | Bug fix | `develop` | `develop` |
| `refactor/<slug>` | Refactoring | `develop` | `develop` |
| `release/<version>` | Release preparation | `develop` | `main` + `develop` |
| `hotfix/<slug>` | Emergency fix | `main` | `main` + `develop` |

## Branch Naming

- Use lowercase, hyphens for spaces
- Include task/issue ID when applicable
- Examples: `feat/42-kanban-board`, `fix/15-websocket-reconnect`, `refactor/simplify-executor`

## Merge Rules

### develop

- **Direct push**: Allowed (small changes, TDD cycle commits, docs)
- **From feature/fix branches**: PR required, squash merge
- **Force push**: Prohibited

### main

- **Direct push**: Prohibited
- **From release branches**: PR required, merge commit, tag with version
- **From hotfix branches**: PR required, merge commit, also merge back to develop
- **Force push**: Prohibited

## Worktree Naming Convention

Use git worktrees for parallel development:

```
../<repo>-<branch-type>-<task-id>
```

Examples:

```
../gantry_board-feat-42       # feature branch worktree
../gantry_board-fix-15        # fix branch worktree
../gantry_board-release-0.2   # release branch worktree
```

## Workflow

### Feature Development

```
git checkout develop
git pull origin develop
git checkout -b feat/<task-id>-<slug>
# ... develop with TDD commits ...
# Create PR to develop (squash merge)
```

### Release

```
git checkout develop
git checkout -b release/<version>
# ... version bump, changelog ...
# Create PR to main (merge commit)
# Tag: v<version>
# Merge back to develop
```

### Hotfix

```
git checkout main
git checkout -b hotfix/<slug>
# ... fix ...
# Create PR to main (merge commit)
# Tag: v<version>
# Merge back to develop
```

## GitHub Branch Protection

| Branch | PR required | Force push | Direct push |
|--------|------------|------------|-------------|
| `main` | Yes | Blocked | Blocked |
| `develop` | No | Blocked | Allowed |

## Scaling Note

When the team grows, consider requiring PRs for all merges to develop.
