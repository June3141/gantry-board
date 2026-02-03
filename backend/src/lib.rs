pub mod config;
pub mod db;
pub mod error;
pub mod handlers;
pub mod models;
pub mod openapi;
pub mod services;
pub mod ws;

use std::sync::Arc;

use axum::routing::{delete, get, patch, post};
use axum::Router;
use sqlx::SqlitePool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::ws::hub::Hub;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub ws_hub: Arc<Hub>,
    pub config: Arc<config::Config>,
}

pub fn app(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/tasks", get(handlers::tasks::list_tasks))
        .route("/tasks", post(handlers::tasks::create_task))
        .route("/tasks/{id}", get(handlers::tasks::get_task))
        .route("/tasks/{id}", patch(handlers::tasks::update_task))
        .route("/tasks/{id}", delete(handlers::tasks::delete_task))
        .route("/projects", get(handlers::projects::list_projects))
        .route("/projects", post(handlers::projects::create_project))
        .route("/projects/{id}", get(handlers::projects::get_project))
        .route("/projects/{id}", patch(handlers::projects::update_project))
        .route("/projects/{id}", delete(handlers::projects::delete_project));

    Router::new()
        .route("/health", get(handlers::health::health_check))
        .route("/ws", get(ws::handler::ws_handler))
        .nest("/api", api_routes)
        .merge(
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()),
        )
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
