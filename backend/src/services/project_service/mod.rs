//! Project service — split into commands (write) and queries (read) submodules.

pub mod commands;
pub mod queries;

// Re-export all public items for backward compatibility.
pub use commands::*;
pub use queries::*;

use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;

use crate::models::project::Project;

/// Internal row type shared between query and command submodules.
#[derive(FromRow)]
pub(crate) struct ProjectRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[sqlx(default)]
    pub repository_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<ProjectRow> for Project {
    type Error = uuid::Error;

    fn try_from(row: ProjectRow) -> Result<Self, Self::Error> {
        Ok(Project {
            id: row.id.parse()?,
            name: row.name,
            description: row.description,
            repository_path: row.repository_path,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}
