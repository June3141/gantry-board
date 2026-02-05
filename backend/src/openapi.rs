use utoipa::OpenApi;

use crate::handlers;
use crate::models;
use crate::sse;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::health::health_check,
        handlers::auth::register,
        handlers::auth::login,
        handlers::auth::logout,
        handlers::auth::me,
        handlers::tasks::list_tasks,
        handlers::tasks::create_task,
        handlers::tasks::get_task,
        handlers::tasks::update_task,
        handlers::tasks::delete_task,
        handlers::projects::list_projects,
        handlers::projects::create_project,
        handlers::projects::get_project,
        handlers::projects::update_project,
        handlers::projects::delete_project,
        handlers::project_members::list_members,
        handlers::project_members::add_member,
        handlers::project_members::get_member,
        handlers::project_members::update_member,
        handlers::project_members::remove_member,
    ),
    components(schemas(
        models::task::Task,
        models::task::TaskStatus,
        models::task::TaskPriority,
        models::task::CreateTaskRequest,
        models::task::UpdateTaskRequest,
        models::project::Project,
        models::project::CreateProjectRequest,
        models::project::UpdateProjectRequest,
        models::project::ProjectMember,
        models::project::MemberRole,
        models::project::AddMemberRequest,
        models::project::UpdateMemberRequest,
        models::user::User,
        models::user::RegisterRequest,
        models::user::LoginRequest,
        models::user::AuthResponse,
        sse::event::SseEvent,
    )),
    tags(
        (name = "health", description = "Health check"),
        (name = "auth", description = "Authentication"),
        (name = "tasks", description = "Task management"),
        (name = "projects", description = "Project management"),
        (name = "project-members", description = "Project member management"),
    ),
    info(
        title = "Gantry Board API",
        version = "0.1.0",
        description = "AI Agent Orchestration Kanban Board API",
        license(name = "Apache-2.0", identifier = "Apache-2.0")
    )
)]
pub struct ApiDoc;
