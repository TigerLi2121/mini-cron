use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: Option<u64>,
    pub app_name: String,
    pub task_name: String,
    pub cron: String,
    pub url: String,
    pub status: i32,
    pub cron_id: Option<u64>,
}
