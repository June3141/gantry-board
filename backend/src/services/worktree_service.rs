use std::path::{Path, PathBuf};

use git2::{BranchType, Repository, WorktreeAddOptions, WorktreePruneOptions};
use tracing::warn;

use crate::error::{AppError, AppResult};

fn validate_worktree_name(name: &str) -> AppResult<()> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
        || name == "."
        || name == ".."
        || name.starts_with('-')
    {
        return Err(AppError::Validation(format!(
            "invalid worktree name: {name}"
        )));
    }
    Ok(())
}

/// Information about a git worktree.
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub name: String,
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_valid: bool,
}

/// Create a new worktree with a branch forked from HEAD.
pub fn create_worktree(repo_path: &Path, name: &str) -> AppResult<WorktreeInfo> {
    validate_worktree_name(name)?;
    let repo = Repository::open(repo_path)?;

    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let mut branch = repo.branch(name, &commit, false)?;

    let wt_path = repo_path
        .parent()
        .ok_or_else(|| AppError::Internal("repository has no parent directory".to_string()))?
        .join(name);

    let mut opts = WorktreeAddOptions::new();
    opts.reference(Some(branch.get()));
    if let Err(e) = repo.worktree(name, &wt_path, Some(&opts)) {
        let _ = branch.delete();
        return Err(e.into());
    }

    Ok(WorktreeInfo {
        name: name.to_string(),
        path: wt_path,
        branch: Some(name.to_string()),
        is_valid: true,
    })
}

/// List all worktrees in the repository.
pub fn list_worktrees(repo_path: &Path) -> AppResult<Vec<WorktreeInfo>> {
    let repo = Repository::open(repo_path)?;
    let worktrees = repo.worktrees()?;

    let mut result = Vec::new();
    for name in worktrees.iter().flatten() {
        let info = build_worktree_info(&repo, name)?;
        result.push(info);
    }
    Ok(result)
}

/// Get information about a specific worktree by name.
pub fn get_worktree(repo_path: &Path, name: &str) -> AppResult<WorktreeInfo> {
    let repo = Repository::open(repo_path)?;
    build_worktree_info(&repo, name)
}

/// Delete a worktree and its working directory.
pub fn delete_worktree(repo_path: &Path, name: &str) -> AppResult<()> {
    let repo = Repository::open(repo_path)?;
    let wt = repo.find_worktree(name)?;

    let mut opts = WorktreePruneOptions::new();
    opts.valid(true);
    opts.working_tree(true);
    wt.prune(Some(&mut opts))?;

    // Clean up the branch
    if let Ok(mut branch) = repo.find_branch(name, BranchType::Local) {
        if let Err(e) = branch.delete() {
            warn!(%e, name, "failed to delete branch after worktree removal");
        }
    }

    Ok(())
}

fn build_worktree_info(repo: &Repository, name: &str) -> AppResult<WorktreeInfo> {
    let wt = repo.find_worktree(name)?;
    let is_valid = wt.validate().is_ok();
    let wt_path = wt.path().to_path_buf();

    let branch = if is_valid {
        Repository::open(&wt_path).ok().and_then(|wt_repo| {
            wt_repo
                .head()
                .ok()
                .and_then(|h| h.shorthand().map(String::from))
        })
    } else {
        None
    };

    Ok(WorktreeInfo {
        name: name.to_string(),
        path: wt_path,
        branch,
        is_valid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Creates a temporary git repository with one initial commit.
    /// The repo is placed in a `repo` subdirectory so worktrees can be
    /// created as siblings inside the same TempDir.
    fn setup_test_repo() -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = dir.path().join("repo");
        std::fs::create_dir(&repo_path).expect("Failed to create repo dir");
        let repo = git2::Repository::init(&repo_path).expect("Failed to init repo");

        let sig =
            git2::Signature::now("Test", "test@example.com").expect("Failed to create signature");
        let tree_id = repo
            .index()
            .expect("Failed to get index")
            .write_tree()
            .expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .expect("Failed to create initial commit");

        (dir, repo_path)
    }

    #[test]

    fn test_create_worktree_returns_info() {
        let (_dir, repo_path) = setup_test_repo();

        let info = create_worktree(&repo_path, "test-wt").expect("Failed to create worktree");

        assert_eq!(info.name, "test-wt");
        assert_eq!(info.branch, Some("test-wt".to_string()));
        assert!(info.is_valid);
    }

    #[test]

    fn test_create_worktree_directory_exists() {
        let (_dir, repo_path) = setup_test_repo();

        let info = create_worktree(&repo_path, "test-wt").expect("Failed to create worktree");

        assert!(info.path.exists());
    }

    #[test]

    fn test_create_duplicate_worktree_returns_error() {
        let (_dir, repo_path) = setup_test_repo();

        create_worktree(&repo_path, "dup-wt").expect("First create should succeed");
        let result = create_worktree(&repo_path, "dup-wt");

        assert!(result.is_err());
    }

    #[test]

    fn test_list_worktrees_empty() {
        let (_dir, repo_path) = setup_test_repo();

        let list = list_worktrees(&repo_path).expect("Failed to list worktrees");

        assert!(list.is_empty());
    }

    #[test]

    fn test_list_worktrees_returns_created() {
        let (_dir, repo_path) = setup_test_repo();

        create_worktree(&repo_path, "wt-a").expect("Failed to create wt-a");
        create_worktree(&repo_path, "wt-b").expect("Failed to create wt-b");

        let list = list_worktrees(&repo_path).expect("Failed to list worktrees");

        assert_eq!(list.len(), 2);
        let names: Vec<&str> = list.iter().map(|w| w.name.as_str()).collect();
        assert!(names.contains(&"wt-a"));
        assert!(names.contains(&"wt-b"));
    }

    #[test]

    fn test_get_worktree_returns_existing() {
        let (_dir, repo_path) = setup_test_repo();

        create_worktree(&repo_path, "my-wt").expect("Failed to create worktree");

        let info = get_worktree(&repo_path, "my-wt").expect("Failed to get worktree");

        assert_eq!(info.name, "my-wt");
        assert!(info.is_valid);
    }

    #[test]

    fn test_get_worktree_not_found() {
        let (_dir, repo_path) = setup_test_repo();

        let result = get_worktree(&repo_path, "nonexistent");

        assert!(result.is_err());
    }

    #[test]

    fn test_delete_worktree_removes_it() {
        let (_dir, repo_path) = setup_test_repo();

        let info = create_worktree(&repo_path, "del-wt").expect("Failed to create worktree");
        let wt_path = info.path.clone();

        delete_worktree(&repo_path, "del-wt").expect("Failed to delete worktree");

        assert!(get_worktree(&repo_path, "del-wt").is_err());
        assert!(!wt_path.exists());
    }

    #[test]
    fn test_create_worktree_rejects_path_traversal() {
        let (_dir, repo_path) = setup_test_repo();

        assert!(create_worktree(&repo_path, "../escape").is_err());
        assert!(create_worktree(&repo_path, "a/b").is_err());
        assert!(create_worktree(&repo_path, "..").is_err());
        assert!(create_worktree(&repo_path, ".").is_err());
        assert!(create_worktree(&repo_path, "").is_err());
    }
}
