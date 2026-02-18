use async_trait::async_trait;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::project::{CreateProjectRequest, Project, UpdateProjectRequest};
use crate::models::task::{CreateTaskRequest, Task, UpdateTaskRequest};

/// Abstraction over task persistence operations.
#[async_trait]
pub trait TaskService: Send + Sync {
    async fn create_task(&self, req: &CreateTaskRequest) -> AppResult<Task>;
    async fn get_task(&self, id: Uuid) -> AppResult<Task>;
    async fn list_tasks(&self, project_id: Uuid) -> AppResult<Vec<Task>>;
    async fn list_tasks_paginated(
        &self,
        project_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> AppResult<(Vec<Task>, i64)>;
    async fn update_task(&self, id: Uuid, req: &UpdateTaskRequest) -> AppResult<Task>;
    async fn delete_task(&self, id: Uuid) -> AppResult<()>;
}

/// Abstraction over project persistence operations.
#[async_trait]
pub trait ProjectService: Send + Sync {
    async fn create_project(&self, req: &CreateProjectRequest) -> AppResult<Project>;
    async fn get_project(&self, id: Uuid) -> AppResult<Project>;
    async fn list_projects(&self) -> AppResult<Vec<Project>>;
    async fn list_projects_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> AppResult<(Vec<Project>, i64)>;
    async fn list_projects_for_user(&self, user_id: Uuid) -> AppResult<Vec<Project>>;
    async fn update_project(&self, id: Uuid, req: &UpdateProjectRequest) -> AppResult<Project>;
    async fn delete_project(&self, id: Uuid) -> AppResult<()>;
}

/// SQLite-backed [`TaskService`] that delegates to free functions.
pub struct SqliteTaskService {
    pool: SqlitePool,
}

impl SqliteTaskService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TaskService for SqliteTaskService {
    async fn create_task(&self, req: &CreateTaskRequest) -> AppResult<Task> {
        super::task_service::create_task(&self.pool, req).await
    }

    async fn get_task(&self, id: Uuid) -> AppResult<Task> {
        super::task_service::get_task(&self.pool, id).await
    }

    async fn list_tasks(&self, project_id: Uuid) -> AppResult<Vec<Task>> {
        super::task_service::list_tasks(&self.pool, project_id).await
    }

    async fn list_tasks_paginated(
        &self,
        project_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> AppResult<(Vec<Task>, i64)> {
        super::task_service::list_tasks_paginated(&self.pool, project_id, limit, offset).await
    }

    async fn update_task(&self, id: Uuid, req: &UpdateTaskRequest) -> AppResult<Task> {
        super::task_service::update_task(&self.pool, id, req).await
    }

    async fn delete_task(&self, id: Uuid) -> AppResult<()> {
        super::task_service::delete_task(&self.pool, id).await
    }
}

/// SQLite-backed [`ProjectService`] that delegates to free functions.
pub struct SqliteProjectService {
    pool: SqlitePool,
}

impl SqliteProjectService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProjectService for SqliteProjectService {
    async fn create_project(&self, req: &CreateProjectRequest) -> AppResult<Project> {
        super::project_service::create_project(&self.pool, req).await
    }

    async fn get_project(&self, id: Uuid) -> AppResult<Project> {
        super::project_service::get_project(&self.pool, id).await
    }

    async fn list_projects(&self) -> AppResult<Vec<Project>> {
        super::project_service::list_projects(&self.pool).await
    }

    async fn list_projects_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> AppResult<(Vec<Project>, i64)> {
        super::project_service::list_projects_paginated(&self.pool, limit, offset).await
    }

    async fn list_projects_for_user(&self, user_id: Uuid) -> AppResult<Vec<Project>> {
        super::project_service::list_projects_for_user(&self.pool, user_id).await
    }

    async fn update_project(&self, id: Uuid, req: &UpdateProjectRequest) -> AppResult<Project> {
        super::project_service::update_project(&self.pool, id, req).await
    }

    async fn delete_project(&self, id: Uuid) -> AppResult<()> {
        super::project_service::delete_project(&self.pool, id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::Project;
    use crate::models::task::{TaskPriority, TaskStatus};
    use chrono::Utc;
    use std::sync::Arc;

    /// Mock implementation demonstrating the DI pattern.
    struct MockTaskService {
        tasks: Vec<Task>,
    }

    #[async_trait]
    impl TaskService for MockTaskService {
        async fn create_task(&self, req: &CreateTaskRequest) -> AppResult<Task> {
            Ok(Task {
                id: Uuid::new_v4(),
                project_id: req.project_id,
                title: req.title.clone(),
                description: req.description.clone(),
                status: req.status.clone().unwrap_or(TaskStatus::Backlog),
                priority: req.priority.clone().unwrap_or(TaskPriority::Medium),
                parent_id: req.parent_id,
                assigned_to: req.assigned_to,
                position: 0,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
        }

        async fn get_task(&self, id: Uuid) -> AppResult<Task> {
            self.tasks
                .iter()
                .find(|t| t.id == id)
                .cloned()
                .ok_or_else(|| crate::error::AppError::NotFound(format!("Task {id}")))
        }

        async fn list_tasks(&self, project_id: Uuid) -> AppResult<Vec<Task>> {
            Ok(self
                .tasks
                .iter()
                .filter(|t| t.project_id == project_id)
                .cloned()
                .collect())
        }

        async fn list_tasks_paginated(
            &self,
            project_id: Uuid,
            limit: i64,
            offset: i64,
        ) -> AppResult<(Vec<Task>, i64)> {
            let filtered: Vec<_> = self
                .tasks
                .iter()
                .filter(|t| t.project_id == project_id)
                .cloned()
                .collect();
            let total = filtered.len() as i64;
            let page = filtered
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .collect();
            Ok((page, total))
        }

        async fn update_task(&self, id: Uuid, _req: &UpdateTaskRequest) -> AppResult<Task> {
            self.get_task(id).await
        }

        async fn delete_task(&self, _id: Uuid) -> AppResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_mock_task_service_create() {
        let svc: Arc<dyn TaskService> = Arc::new(MockTaskService { tasks: vec![] });

        let project_id = Uuid::new_v4();
        let req = CreateTaskRequest {
            project_id,
            title: "Test task".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        };

        let task = svc.create_task(&req).await.unwrap();
        assert_eq!(task.title, "Test task");
        assert_eq!(task.project_id, project_id);
    }

    #[tokio::test]
    async fn test_mock_task_service_list() {
        let project_id = Uuid::new_v4();
        let task = Task {
            id: Uuid::new_v4(),
            project_id,
            title: "T1".to_string(),
            description: None,
            status: TaskStatus::Backlog,
            priority: TaskPriority::Medium,
            parent_id: None,
            assigned_to: None,
            position: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let svc: Arc<dyn TaskService> = Arc::new(MockTaskService {
            tasks: vec![task.clone()],
        });

        let tasks = svc.list_tasks(project_id).await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, task.id);
    }

    #[tokio::test]
    async fn test_mock_task_service_not_found() {
        let svc: Arc<dyn TaskService> = Arc::new(MockTaskService { tasks: vec![] });
        let result = svc.get_task(Uuid::new_v4()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_trait_object_can_be_stored_in_arc() {
        // Verify Send + Sync bounds allow Arc<dyn Trait> usage
        let svc: Arc<dyn TaskService> = Arc::new(MockTaskService { tasks: vec![] });
        let svc_clone = Arc::clone(&svc);

        let handle = tokio::spawn(async move {
            let project_id = Uuid::new_v4();
            svc_clone.list_tasks(project_id).await.unwrap()
        });

        let result = handle.await.unwrap();
        assert!(result.is_empty());

        // Original still usable
        assert!(svc.list_tasks(Uuid::new_v4()).await.unwrap().is_empty());
    }

    // ------------------------------------------------------------------
    // MockProjectService
    // ------------------------------------------------------------------

    /// Mock implementation demonstrating the DI pattern for projects.
    struct MockProjectService {
        projects: Vec<Project>,
    }

    #[async_trait]
    impl ProjectService for MockProjectService {
        async fn create_project(&self, req: &CreateProjectRequest) -> AppResult<Project> {
            Ok(Project {
                id: Uuid::new_v4(),
                name: req.name.clone(),
                description: req.description.clone(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
        }

        async fn get_project(&self, id: Uuid) -> AppResult<Project> {
            self.projects
                .iter()
                .find(|p| p.id == id)
                .cloned()
                .ok_or_else(|| crate::error::AppError::NotFound(format!("Project {id}")))
        }

        async fn list_projects(&self) -> AppResult<Vec<Project>> {
            Ok(self.projects.clone())
        }

        async fn list_projects_paginated(
            &self,
            limit: i64,
            offset: i64,
        ) -> AppResult<(Vec<Project>, i64)> {
            let total = self.projects.len() as i64;
            let page = self
                .projects
                .iter()
                .skip(offset as usize)
                .take(limit as usize)
                .cloned()
                .collect();
            Ok((page, total))
        }

        async fn list_projects_for_user(&self, _user_id: Uuid) -> AppResult<Vec<Project>> {
            Ok(self.projects.clone())
        }

        async fn update_project(
            &self,
            id: Uuid,
            _req: &UpdateProjectRequest,
        ) -> AppResult<Project> {
            self.get_project(id).await
        }

        async fn delete_project(&self, _id: Uuid) -> AppResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_mock_project_service_create() {
        let svc: Arc<dyn ProjectService> = Arc::new(MockProjectService { projects: vec![] });

        let req = CreateProjectRequest {
            name: "Test project".to_string(),
            description: Some("A description".to_string()),
        };

        let project = svc.create_project(&req).await.unwrap();
        assert_eq!(project.name, "Test project");
        assert_eq!(project.description, Some("A description".to_string()));
    }

    #[tokio::test]
    async fn test_mock_project_service_list() {
        let project = Project {
            id: Uuid::new_v4(),
            name: "P1".to_string(),
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let svc: Arc<dyn ProjectService> = Arc::new(MockProjectService {
            projects: vec![project.clone()],
        });

        let projects = svc.list_projects().await.unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, project.id);
    }

    #[tokio::test]
    async fn test_mock_project_service_not_found() {
        let svc: Arc<dyn ProjectService> = Arc::new(MockProjectService { projects: vec![] });
        let result = svc.get_project(Uuid::new_v4()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_project_trait_object_can_be_stored_in_arc() {
        let svc: Arc<dyn ProjectService> = Arc::new(MockProjectService { projects: vec![] });
        let svc_clone = Arc::clone(&svc);

        let handle = tokio::spawn(async move { svc_clone.list_projects().await.unwrap() });

        let result = handle.await.unwrap();
        assert!(result.is_empty());

        // Original still usable
        assert!(svc.list_projects().await.unwrap().is_empty());
    }
}
