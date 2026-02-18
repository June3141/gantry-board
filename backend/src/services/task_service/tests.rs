use super::*;
use crate::models::project::{AddMemberRequest, CreateProjectRequest, MemberRole};
use crate::models::user::RegisterRequest;
use crate::services::{member_service, project_service, user_service};
use crate::test_helpers::setup_test_db;

async fn create_test_project(pool: &SqlitePool) -> Uuid {
    let req = CreateProjectRequest {
        name: "Test Project".to_string(),
        description: None,
    };
    let project = project_service::create_project(pool, &req)
        .await
        .expect("Failed to create project");
    project.id
}

async fn create_test_user(pool: &SqlitePool, email: &str) -> Uuid {
    let req = RegisterRequest {
        email: email.to_string(),
        name: email.split('@').next().unwrap().to_string(),
        password: "password123".to_string(),
    };
    user_service::create_user(pool, &req)
        .await
        .expect("Failed to create user")
        .id
}

async fn add_project_member(pool: &SqlitePool, project_id: Uuid, user_id: Uuid) {
    let req = AddMemberRequest {
        user_id,
        role: MemberRole::Member,
    };
    member_service::add_member(pool, project_id, &req)
        .await
        .expect("Failed to add member");
}

#[tokio::test]
async fn test_create_task_saves_to_db_and_returns() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;

    let req = CreateTaskRequest {
        project_id,
        title: "Test Task".to_string(),
        description: Some("A test task".to_string()),
        status: None,
        priority: None,
        parent_id: None,
        assigned_to: None,
    };
    let task = create_task(&pool, &req)
        .await
        .expect("Failed to create task");

    assert_eq!(task.title, "Test Task");
    assert_eq!(task.description, Some("A test task".to_string()));
    assert_eq!(task.project_id, project_id);
    assert!(matches!(task.status, TaskStatus::Backlog));
    assert!(matches!(task.priority, TaskPriority::Medium));
}

#[tokio::test]
async fn test_create_task_with_custom_status_and_priority() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;

    let req = CreateTaskRequest {
        project_id,
        title: "Urgent Task".to_string(),
        description: None,
        status: Some(TaskStatus::InProgress),
        priority: Some(TaskPriority::Urgent),
        parent_id: None,
        assigned_to: None,
    };
    let task = create_task(&pool, &req)
        .await
        .expect("Failed to create task");

    assert!(matches!(task.status, TaskStatus::InProgress));
    assert!(matches!(task.priority, TaskPriority::Urgent));
}

#[tokio::test]
async fn test_get_task_returns_existing_task() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;

    let req = CreateTaskRequest {
        project_id,
        title: "Get Me".to_string(),
        description: None,
        status: None,
        priority: None,
        parent_id: None,
        assigned_to: None,
    };
    let created = create_task(&pool, &req)
        .await
        .expect("Failed to create task");

    let task = get_task(&pool, created.id)
        .await
        .expect("Failed to get task");

    assert_eq!(task.id, created.id);
    assert_eq!(task.title, "Get Me");
}

#[tokio::test]
async fn test_get_task_returns_not_found_for_nonexistent() {
    let pool = setup_test_db().await;
    let random_id = Uuid::new_v4();

    let result = get_task(&pool, random_id).await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_list_tasks_returns_empty_when_no_tasks() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;

    let tasks = list_tasks(&pool, project_id)
        .await
        .expect("Failed to list tasks");

    assert!(tasks.is_empty());
}

#[tokio::test]
async fn test_list_tasks_returns_tasks_for_project() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;

    create_task(
        &pool,
        &CreateTaskRequest {
            project_id,
            title: "Task 1".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        },
    )
    .await
    .expect("Failed to create task 1");

    create_task(
        &pool,
        &CreateTaskRequest {
            project_id,
            title: "Task 2".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        },
    )
    .await
    .expect("Failed to create task 2");

    let tasks = list_tasks(&pool, project_id)
        .await
        .expect("Failed to list tasks");

    assert_eq!(tasks.len(), 2);
}

#[tokio::test]
async fn test_list_tasks_filters_by_project() {
    let pool = setup_test_db().await;
    let project1 = create_test_project(&pool).await;
    let project2 = create_test_project(&pool).await;

    create_task(
        &pool,
        &CreateTaskRequest {
            project_id: project1,
            title: "Project 1 Task".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        },
    )
    .await
    .expect("Failed to create task");

    create_task(
        &pool,
        &CreateTaskRequest {
            project_id: project2,
            title: "Project 2 Task".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        },
    )
    .await
    .expect("Failed to create task");

    let tasks1 = list_tasks(&pool, project1).await.expect("Failed to list");
    let tasks2 = list_tasks(&pool, project2).await.expect("Failed to list");

    assert_eq!(tasks1.len(), 1);
    assert_eq!(tasks1[0].title, "Project 1 Task");
    assert_eq!(tasks2.len(), 1);
    assert_eq!(tasks2[0].title, "Project 2 Task");
}

#[tokio::test]
async fn test_update_task_changes_title() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;

    let created = create_task(
        &pool,
        &CreateTaskRequest {
            project_id,
            title: "Original".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        },
    )
    .await
    .expect("Failed to create task");

    let updated = update_task(
        &pool,
        created.id,
        &UpdateTaskRequest {
            title: Some("Updated".to_string()),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
            position: None,
        },
    )
    .await
    .expect("Failed to update task");

    assert_eq!(updated.title, "Updated");
}

#[tokio::test]
async fn test_update_task_changes_status() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;

    let created = create_task(
        &pool,
        &CreateTaskRequest {
            project_id,
            title: "Task".to_string(),
            description: None,
            status: Some(TaskStatus::Backlog),
            priority: None,
            parent_id: None,
            assigned_to: None,
        },
    )
    .await
    .expect("Failed to create task");

    let updated = update_task(
        &pool,
        created.id,
        &UpdateTaskRequest {
            title: None,
            description: None,
            status: Some(TaskStatus::Done),
            priority: None,
            parent_id: None,
            assigned_to: None,
            position: None,
        },
    )
    .await
    .expect("Failed to update task");

    assert!(matches!(updated.status, TaskStatus::Done));
}

#[tokio::test]
async fn test_update_task_changes_position() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;

    let created = create_task(
        &pool,
        &CreateTaskRequest {
            project_id,
            title: "Task".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        },
    )
    .await
    .expect("Failed to create task");

    let updated = update_task(
        &pool,
        created.id,
        &UpdateTaskRequest {
            title: None,
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
            position: Some(5),
        },
    )
    .await
    .expect("Failed to update task");

    assert_eq!(updated.position, 5);
}

#[tokio::test]
async fn test_delete_task_removes_from_db() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;

    let created = create_task(
        &pool,
        &CreateTaskRequest {
            project_id,
            title: "To Delete".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        },
    )
    .await
    .expect("Failed to create task");

    delete_task(&pool, created.id)
        .await
        .expect("Failed to delete task");

    let result = get_task(&pool, created.id).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_delete_nonexistent_task_returns_not_found() {
    let pool = setup_test_db().await;
    let random_id = Uuid::new_v4();

    let result = delete_task(&pool, random_id).await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_list_tasks_paginated_returns_total_and_data() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;

    for i in 0..5 {
        create_task(
            &pool,
            &CreateTaskRequest {
                project_id,
                title: format!("Task {}", i),
                description: None,
                status: None,
                priority: None,
                parent_id: None,
                assigned_to: None,
            },
        )
        .await
        .expect("Failed to create task");
    }

    let (tasks, total) = list_tasks_paginated(&pool, project_id, 2, 0)
        .await
        .expect("Failed to list tasks paginated");

    assert_eq!(tasks.len(), 2);
    assert_eq!(total, 5);
}

#[tokio::test]
async fn test_list_tasks_paginated_respects_offset() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;

    for i in 0..5 {
        create_task(
            &pool,
            &CreateTaskRequest {
                project_id,
                title: format!("Task {}", i),
                description: None,
                status: None,
                priority: None,
                parent_id: None,
                assigned_to: None,
            },
        )
        .await
        .expect("Failed to create task");
    }

    let (tasks, total) = list_tasks_paginated(&pool, project_id, 2, 3)
        .await
        .expect("Failed to list tasks paginated");

    assert_eq!(tasks.len(), 2);
    assert_eq!(total, 5);
}

#[tokio::test]
async fn test_list_tasks_paginated_offset_beyond_total() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;

    create_task(
        &pool,
        &CreateTaskRequest {
            project_id,
            title: "Only Task".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        },
    )
    .await
    .expect("Failed to create task");

    let (tasks, total) = list_tasks_paginated(&pool, project_id, 10, 100)
        .await
        .expect("Failed to list tasks paginated");

    assert!(tasks.is_empty());
    assert_eq!(total, 1);
}

#[tokio::test]
async fn test_create_task_with_nonexistent_assigned_to_returns_validation_error() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;
    let nonexistent_user = Uuid::new_v4();

    let req = CreateTaskRequest {
        project_id,
        title: "Task".to_string(),
        description: None,
        status: None,
        priority: None,
        parent_id: None,
        assigned_to: Some(nonexistent_user),
    };
    let result = create_task(&pool, &req).await;

    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn test_create_task_with_parent_in_different_project_returns_validation_error() {
    let pool = setup_test_db().await;
    let project1 = create_test_project(&pool).await;
    let project2 = create_test_project(&pool).await;

    let parent = create_task(
        &pool,
        &CreateTaskRequest {
            project_id: project1,
            title: "Parent".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        },
    )
    .await
    .expect("parent task creation should succeed");

    let req = CreateTaskRequest {
        project_id: project2,
        title: "Child".to_string(),
        description: None,
        status: None,
        priority: None,
        parent_id: Some(parent.id),
        assigned_to: None,
    };
    let result = create_task(&pool, &req).await;

    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn test_update_task_with_nonexistent_assigned_to_returns_validation_error() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;
    let nonexistent_user = Uuid::new_v4();

    let created = create_task(
        &pool,
        &CreateTaskRequest {
            project_id,
            title: "Task".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        },
    )
    .await
    .expect("task creation should succeed");

    let result = update_task(
        &pool,
        created.id,
        &UpdateTaskRequest {
            title: None,
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: Some(nonexistent_user),
            position: None,
        },
    )
    .await;

    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn test_update_task_validation_failure_preserves_original() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;
    let nonexistent_user = Uuid::new_v4();

    let created = create_task(
        &pool,
        &CreateTaskRequest {
            project_id,
            title: "Original Title".to_string(),
            description: Some("Original Description".to_string()),
            status: Some(TaskStatus::Backlog),
            priority: Some(TaskPriority::Low),
            parent_id: None,
            assigned_to: None,
        },
    )
    .await
    .expect("task creation should succeed");

    let result = update_task(
        &pool,
        created.id,
        &UpdateTaskRequest {
            title: Some("Changed Title".to_string()),
            description: Some("Changed Description".to_string()),
            status: Some(TaskStatus::Done),
            priority: Some(TaskPriority::Urgent),
            parent_id: None,
            assigned_to: Some(nonexistent_user),
            position: Some(99),
        },
    )
    .await;

    assert!(matches!(result, Err(AppError::Validation(_))));

    let after = get_task(&pool, created.id)
        .await
        .expect("task should still exist");
    assert_eq!(after.title, "Original Title");
    assert_eq!(after.description, Some("Original Description".to_string()));
    assert!(matches!(after.status, TaskStatus::Backlog));
    assert!(matches!(after.priority, TaskPriority::Low));
    assert_eq!(after.position, 0);
    assert!(after.assigned_to.is_none());
}

#[tokio::test]
async fn test_create_task_validation_failure_leaves_no_row() {
    let pool = setup_test_db().await;
    let project1 = create_test_project(&pool).await;
    let project2 = create_test_project(&pool).await;

    let parent = create_task(
        &pool,
        &CreateTaskRequest {
            project_id: project2,
            title: "Parent in P2".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        },
    )
    .await
    .expect("parent task creation should succeed");

    let result = create_task(
        &pool,
        &CreateTaskRequest {
            project_id: project1,
            title: "Orphan".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: Some(parent.id),
            assigned_to: None,
        },
    )
    .await;

    assert!(matches!(result, Err(AppError::Validation(_))));

    let tasks = list_tasks(&pool, project1)
        .await
        .expect("list should succeed");
    assert!(tasks.is_empty(), "no task should exist after failed create");
}

#[tokio::test]
async fn test_update_task_with_parent_in_different_project_returns_validation_error() {
    let pool = setup_test_db().await;
    let project1 = create_test_project(&pool).await;
    let project2 = create_test_project(&pool).await;

    let parent_in_p2 = create_task(
        &pool,
        &CreateTaskRequest {
            project_id: project2,
            title: "Parent in project 2".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        },
    )
    .await
    .expect("parent task creation should succeed");

    let child = create_task(
        &pool,
        &CreateTaskRequest {
            project_id: project1,
            title: "Child in project 1".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        },
    )
    .await
    .expect("child task creation should succeed");

    let result = update_task(
        &pool,
        child.id,
        &UpdateTaskRequest {
            title: None,
            description: None,
            status: None,
            priority: None,
            parent_id: Some(parent_in_p2.id),
            assigned_to: None,
            position: None,
        },
    )
    .await;

    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn test_create_task_assigned_to_non_member_returns_validation_error() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;
    // User exists but is NOT a member of the project
    let non_member = create_test_user(&pool, "outsider@test.com").await;

    let req = CreateTaskRequest {
        project_id,
        title: "Task".to_string(),
        description: None,
        status: None,
        priority: None,
        parent_id: None,
        assigned_to: Some(non_member),
    };
    let result = create_task(&pool, &req).await;

    assert!(
        matches!(result, Err(AppError::Validation(ref msg)) if msg.contains("not a member")),
        "expected Validation error about non-member, got: {result:?}"
    );
}

#[tokio::test]
async fn test_create_task_assigned_to_project_member_succeeds() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;
    let member = create_test_user(&pool, "member@test.com").await;
    add_project_member(&pool, project_id, member).await;

    let req = CreateTaskRequest {
        project_id,
        title: "Task".to_string(),
        description: None,
        status: None,
        priority: None,
        parent_id: None,
        assigned_to: Some(member),
    };
    let task = create_task(&pool, &req)
        .await
        .expect("assigning to project member should succeed");

    assert_eq!(task.assigned_to, Some(member));
}

#[tokio::test]
async fn test_update_task_assigned_to_non_member_returns_validation_error() {
    let pool = setup_test_db().await;
    let project_id = create_test_project(&pool).await;
    let non_member = create_test_user(&pool, "outsider@test.com").await;

    let created = create_task(
        &pool,
        &CreateTaskRequest {
            project_id,
            title: "Task".to_string(),
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: None,
        },
    )
    .await
    .expect("task creation should succeed");

    let result = update_task(
        &pool,
        created.id,
        &UpdateTaskRequest {
            title: None,
            description: None,
            status: None,
            priority: None,
            parent_id: None,
            assigned_to: Some(non_member),
            position: None,
        },
    )
    .await;

    assert!(
        matches!(result, Err(AppError::Validation(ref msg)) if msg.contains("not a member")),
        "expected Validation error about non-member, got: {result:?}"
    );
}
