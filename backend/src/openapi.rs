use utoipa::OpenApi;

use crate::handlers;
use crate::models;
use crate::ws;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::health::health_check,
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
        models::user::User,
        ws::message::WsMessage,
    )),
    tags(
        (name = "health", description = "Health check"),
        (name = "tasks", description = "Task management"),
        (name = "projects", description = "Project management"),
    ),
    info(
        title = "Gantry Board API",
        version = "0.1.0",
        description = "AI Agent Orchestration Kanban Board API",
        license(name = "Apache-2.0", identifier = "Apache-2.0")
    )
)]
pub struct ApiDoc;
