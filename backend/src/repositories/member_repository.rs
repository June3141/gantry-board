use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use sqlx::sqlite::SqliteConnection;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project::{MemberRole, ProjectMember};

#[derive(FromRow)]
struct MemberRow {
    project_id: String,
    user_id: String,
    role: MemberRole,
    user_name: String,
    user_email: String,
    created_at: DateTime<Utc>,
}

impl TryFrom<MemberRow> for ProjectMember {
    type Error = uuid::Error;

    fn try_from(row: MemberRow) -> Result<Self, Self::Error> {
        Ok(ProjectMember {
            project_id: row.project_id.parse()?,
            user_id: row.user_id.parse()?,
            role: row.role,
            user_name: row.user_name,
            user_email: row.user_email,
            created_at: row.created_at,
        })
    }
}

fn row_to_member(row: MemberRow) -> AppResult<ProjectMember> {
    row.try_into()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

pub async fn insert_tx(
    conn: &mut SqliteConnection,
    project_id: Uuid,
    user_id: Uuid,
    role: &MemberRole,
    now: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO project_members (project_id, user_id, role, created_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(project_id.to_string())
    .bind(user_id.to_string())
    .bind(role)
    .bind(now)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn find_by_project_and_user(
    pool: &SqlitePool,
    project_id: Uuid,
    user_id: Uuid,
) -> AppResult<Option<ProjectMember>> {
    let row = sqlx::query_as::<_, MemberRow>(
        r#"
        SELECT pm.project_id, pm.user_id, pm.role,
               u.name as user_name, u.email as user_email,
               pm.created_at
        FROM project_members pm
        JOIN users u ON pm.user_id = u.id
        WHERE pm.project_id = $1 AND pm.user_id = $2
        "#,
    )
    .bind(project_id.to_string())
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await?;

    row.map(row_to_member).transpose()
}

pub async fn find_all_by_project(
    pool: &SqlitePool,
    project_id: Uuid,
) -> AppResult<Vec<ProjectMember>> {
    let rows = sqlx::query_as::<_, MemberRow>(
        r#"
        SELECT pm.project_id, pm.user_id, pm.role,
               u.name as user_name, u.email as user_email,
               pm.created_at
        FROM project_members pm
        JOIN users u ON pm.user_id = u.id
        WHERE pm.project_id = $1
        ORDER BY pm.created_at ASC
        "#,
    )
    .bind(project_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_member).collect()
}

pub async fn update_role(
    pool: &SqlitePool,
    project_id: Uuid,
    user_id: Uuid,
    role: &MemberRole,
) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE project_members
        SET role = $1
        WHERE project_id = $2 AND user_id = $3
        "#,
    )
    .bind(role)
    .bind(project_id.to_string())
    .bind(user_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete(pool: &SqlitePool, project_id: Uuid, user_id: Uuid) -> AppResult<u64> {
    let result = sqlx::query("DELETE FROM project_members WHERE project_id = $1 AND user_id = $2")
        .bind(project_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

pub async fn find_role(
    pool: &SqlitePool,
    project_id: Uuid,
    user_id: Uuid,
) -> AppResult<Option<MemberRole>> {
    let row = sqlx::query_scalar::<_, String>(
        "SELECT role FROM project_members WHERE project_id = $1 AND user_id = $2",
    )
    .bind(project_id.to_string())
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await?;

    row.map(|s| {
        serde_json::from_value(serde_json::Value::String(s))
            .map_err(|e| AppError::Internal(e.to_string()))
    })
    .transpose()
}

pub async fn has_any_membership(pool: &SqlitePool, user_id: Uuid) -> AppResult<bool> {
    let exists: Option<(i32,)> =
        sqlx::query_as("SELECT 1 FROM project_members WHERE user_id = $1 LIMIT 1")
            .bind(user_id.to_string())
            .fetch_optional(pool)
            .await?;

    Ok(exists.is_some())
}

pub async fn list_project_ids_by_user(pool: &SqlitePool, user_id: Uuid) -> AppResult<Vec<Uuid>> {
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT project_id FROM project_members WHERE user_id = $1",
    )
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|s| {
            s.parse()
                .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
        })
        .collect()
}

pub async fn count_owners(pool: &SqlitePool, project_id: Uuid) -> AppResult<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM project_members WHERE project_id = $1 AND role = 'owner'",
    )
    .bind(project_id.to_string())
    .fetch_one(pool)
    .await?;

    Ok(count)
}

pub async fn user_exists_tx(conn: &mut SqliteConnection, user_id: Uuid) -> AppResult<bool> {
    let exists: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM users WHERE id = $1")
        .bind(user_id.to_string())
        .fetch_optional(&mut *conn)
        .await?;

    Ok(exists.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::CreateProjectRequest;
    use crate::models::user::RegisterRequest;
    use crate::services::{project_service, user_service};
    use crate::test_helpers::setup_test_db;

    async fn create_test_project(pool: &SqlitePool) -> Uuid {
        project_service::create_project(
            pool,
            &CreateProjectRequest {
                name: "Test Project".to_string(),
                description: None,
                repository_path: None,
            },
        )
        .await
        .expect("create project")
        .id
    }

    async fn create_test_user(pool: &SqlitePool) -> Uuid {
        let req = RegisterRequest {
            email: format!("test-{}@example.com", Uuid::new_v4()),
            name: "Test User".to_string(),
            password: "correct horse battery staple purple".to_string(),
        };
        user_service::create_user(pool, &req)
            .await
            .expect("create user")
            .id
    }

    #[tokio::test]
    async fn test_insert_and_find() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = create_test_user(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        insert_tx(
            &mut tx,
            project_id,
            user_id,
            &MemberRole::Member,
            Utc::now(),
        )
        .await
        .expect("insert");
        tx.commit().await.unwrap();

        let member = find_by_project_and_user(&pool, project_id, user_id)
            .await
            .expect("find")
            .expect("should exist");
        assert_eq!(member.project_id, project_id);
        assert_eq!(member.user_id, user_id);
        assert!(matches!(member.role, MemberRole::Member));
    }

    #[tokio::test]
    async fn test_find_returns_none_for_nonexistent() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let result = find_by_project_and_user(&pool, project_id, Uuid::new_v4())
            .await
            .expect("find");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_all_by_project() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let u1 = create_test_user(&pool).await;
        let u2 = create_test_user(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        insert_tx(&mut tx, project_id, u1, &MemberRole::Owner, Utc::now())
            .await
            .unwrap();
        insert_tx(&mut tx, project_id, u2, &MemberRole::Member, Utc::now())
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let members = find_all_by_project(&pool, project_id)
            .await
            .expect("find all");
        assert_eq!(members.len(), 2);
    }

    #[tokio::test]
    async fn test_update_role() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = create_test_user(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        insert_tx(
            &mut tx,
            project_id,
            user_id,
            &MemberRole::Member,
            Utc::now(),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        update_role(&pool, project_id, user_id, &MemberRole::Admin)
            .await
            .expect("update");

        let member = find_by_project_and_user(&pool, project_id, user_id)
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(member.role, MemberRole::Admin));
    }

    #[tokio::test]
    async fn test_delete() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = create_test_user(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        insert_tx(
            &mut tx,
            project_id,
            user_id,
            &MemberRole::Member,
            Utc::now(),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let rows = delete(&pool, project_id, user_id).await.expect("delete");
        assert_eq!(rows, 1);

        assert!(find_by_project_and_user(&pool, project_id, user_id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_find_role() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = create_test_user(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        insert_tx(&mut tx, project_id, user_id, &MemberRole::Admin, Utc::now())
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let role = find_role(&pool, project_id, user_id)
            .await
            .expect("find_role")
            .expect("should have role");
        assert_eq!(role, MemberRole::Admin);

        assert!(find_role(&pool, project_id, Uuid::new_v4())
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_has_any_membership() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = create_test_user(&pool).await;

        assert!(!has_any_membership(&pool, user_id).await.unwrap());

        let mut tx = pool.begin().await.unwrap();
        insert_tx(
            &mut tx,
            project_id,
            user_id,
            &MemberRole::Member,
            Utc::now(),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        assert!(has_any_membership(&pool, user_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_list_project_ids_by_user() {
        let pool = setup_test_db().await;
        let p1 = create_test_project(&pool).await;
        let p2 = create_test_project(&pool).await;
        let user_id = create_test_user(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        insert_tx(&mut tx, p1, user_id, &MemberRole::Owner, Utc::now())
            .await
            .unwrap();
        insert_tx(&mut tx, p2, user_id, &MemberRole::Member, Utc::now())
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let ids = list_project_ids_by_user(&pool, user_id).await.unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&p1));
        assert!(ids.contains(&p2));
    }

    #[tokio::test]
    async fn test_count_owners() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let u1 = create_test_user(&pool).await;
        let u2 = create_test_user(&pool).await;
        let u3 = create_test_user(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        insert_tx(&mut tx, project_id, u1, &MemberRole::Owner, Utc::now())
            .await
            .unwrap();
        insert_tx(&mut tx, project_id, u2, &MemberRole::Owner, Utc::now())
            .await
            .unwrap();
        insert_tx(&mut tx, project_id, u3, &MemberRole::Admin, Utc::now())
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let count = count_owners(&pool, project_id).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_user_exists_tx() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        assert!(user_exists_tx(&mut tx, user_id).await.unwrap());
        assert!(!user_exists_tx(&mut tx, Uuid::new_v4()).await.unwrap());
        tx.commit().await.unwrap();
    }
}
