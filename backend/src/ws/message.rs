use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::task::Task;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum WsMessage {
    TaskCreated { task: Task },
    TaskUpdated { task: Task },
    TaskDeleted { task_id: Uuid },
}
