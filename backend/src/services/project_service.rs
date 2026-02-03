use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project::{CreateProjectRequest, Project, UpdateProjectRequest};

#[derive(FromRow)]
struct ProjectRow {
    id: String,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<ProjectRow> for Project {
    type Error = uuid::Error;

    fn try_from(row: ProjectRow) -> Result<Self, Self::Error> {
        Ok(Project {
            id: row.id.parse()?,
            name: row.name,
            description: row.description,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub async fn create_project(pool: &SqlitePool, req: &CreateProjectRequest) -> AppResult<Project> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO projects (id, name, description, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(id.to_string())
    .bind(&req.name)
    .bind(&req.description)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Project {
        id,
        name: req.name.clone(),
        description: req.description.clone(),
        created_at: now,
        updated_at: now,
    })
}

pub async fn get_project(pool: &SqlitePool, id: Uuid) -> AppResult<Project> {
    let row = sqlx::query_as::<_, ProjectRow>(
        r#"
        SELECT id, name, description, created_at, updated_at
        FROM projects
        WHERE id = $1
        "#,
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(e.into()))?;

    row.map(|r| r.try_into())
        .transpose()
        .map_err(|e: uuid::Error| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::NotFound(format!("project {} not found", id)))
}

pub async fn list_projects(pool: &SqlitePool) -> AppResult<Vec<Project>> {
    let rows = sqlx::query_as::<_, ProjectRow>(
        r#"
        SELECT id, name, description, created_at, updated_at
        FROM projects
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(e.into()))?;

    rows.into_iter()
        .map(|r| r.try_into())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e: uuid::Error| AppError::Internal(e.into()))
}

pub async fn update_project(
    pool: &SqlitePool,
    id: Uuid,
    req: &UpdateProjectRequest,
) -> AppResult<Project> {
    let existing = get_project(pool, id).await?;
    let now = Utc::now();

    let name = req.name.as_ref().unwrap_or(&existing.name);
    let description = req.description.as_ref().or(existing.description.as_ref());

    sqlx::query(
        r#"
        UPDATE projects
        SET name = $1, description = $2, updated_at = $3
        WHERE id = $4
        "#,
    )
    .bind(name)
    .bind(description)
    .bind(now)
    .bind(id.to_string())
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Project {
        id,
        name: name.clone(),
        description: description.cloned(),
        created_at: existing.created_at,
        updated_at: now,
    })
}

pub async fn delete_project(pool: &SqlitePool, id: Uuid) -> AppResult<()> {
    let result = sqlx::query(
        r#"
        DELETE FROM projects
        WHERE id = $1
        "#,
    )
    .bind(id.to_string())
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(e.into()))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("project {} not found", id)));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        pool
    }

    #[tokio::test]
    async fn test_create_project_saves_to_db_and_returns() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: Some("A test project".to_string()),
        };

        let project = create_project(&pool, &req)
            .await
            .expect("Failed to create project");

        assert_eq!(project.name, "Test Project");
        assert_eq!(project.description, Some("A test project".to_string()));
        assert!(!project.id.is_nil());
    }

    #[tokio::test]
    async fn test_get_project_returns_existing_project() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: None,
        };
        let created = create_project(&pool, &req)
            .await
            .expect("Failed to create project");

        let found = get_project(&pool, created.id)
            .await
            .expect("Failed to get project");

        assert_eq!(found.id, created.id);
        assert_eq!(found.name, "Test Project");
    }

    #[tokio::test]
    async fn test_get_project_returns_not_found_for_nonexistent() {
        let pool = setup_test_db().await;
        let random_id = Uuid::new_v4();

        let result = get_project(&pool, random_id).await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_projects_returns_empty_when_no_projects() {
        let pool = setup_test_db().await;

        let projects = list_projects(&pool).await.expect("Failed to list projects");

        assert!(projects.is_empty());
    }

    #[tokio::test]
    async fn test_list_projects_returns_multiple_projects() {
        let pool = setup_test_db().await;
        let req1 = CreateProjectRequest {
            name: "Project 1".to_string(),
            description: None,
        };
        let req2 = CreateProjectRequest {
            name: "Project 2".to_string(),
            description: None,
        };
        create_project(&pool, &req1)
            .await
            .expect("Failed to create project 1");
        create_project(&pool, &req2)
            .await
            .expect("Failed to create project 2");

        let projects = list_projects(&pool).await.expect("Failed to list projects");

        assert_eq!(projects.len(), 2);
    }

    #[tokio::test]
    async fn test_update_project_changes_name() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "Original Name".to_string(),
            description: None,
        };
        let created = create_project(&pool, &req)
            .await
            .expect("Failed to create project");

        let update_req = UpdateProjectRequest {
            name: Some("Updated Name".to_string()),
            description: None,
        };
        let updated = update_project(&pool, created.id, &update_req)
            .await
            .expect("Failed to update project");

        assert_eq!(updated.name, "Updated Name");
    }

    #[tokio::test]
    async fn test_update_project_changes_description() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: Some("Original description".to_string()),
        };
        let created = create_project(&pool, &req)
            .await
            .expect("Failed to create project");

        let update_req = UpdateProjectRequest {
            name: None,
            description: Some("Updated description".to_string()),
        };
        let updated = update_project(&pool, created.id, &update_req)
            .await
            .expect("Failed to update project");

        assert_eq!(updated.description, Some("Updated description".to_string()));
    }

    #[tokio::test]
    async fn test_delete_project_removes_from_db() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "To Be Deleted".to_string(),
            description: None,
        };
        let created = create_project(&pool, &req)
            .await
            .expect("Failed to create project");

        delete_project(&pool, created.id)
            .await
            .expect("Failed to delete project");

        let result = get_project(&pool, created.id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_nonexistent_project_returns_not_found() {
        let pool = setup_test_db().await;
        let random_id = Uuid::new_v4();

        let result = delete_project(&pool, random_id).await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
