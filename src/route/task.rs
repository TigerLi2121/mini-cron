use std::collections::HashMap;

use crate::AppState;
use crate::common::req::Page;
use crate::common::res::{R, RP};
use crate::model::task::Task;
use axum::extract::{Query, State};
use axum::{
    Router,
    extract::Json,
    routing::{delete, get, post},
};
use serde_json::Value;
use tokio::sync::Mutex;
use tracing::{info, warn};

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(page).post(sou).delete(del))
}

use std::sync::LazyLock;

static TASKS: LazyLock<Mutex<HashMap<u64, Task>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

async fn page(_: State<AppState>, Json(m): Json<Task>, Json(page): Json<Page>) -> RP<Vec<Task>> {
    let tasks = TASKS.lock().await;
    let filtered_tasks: Vec<Task> = tasks
        .values()
        .filter(|t| {
            (!m.app_name.is_empty() && t.app_name.contains(&m.app_name))
                || (!m.task_name.is_empty() && t.task_name.contains(&m.task_name))
        })
        .cloned()
        .collect();

    let total = filtered_tasks.len();
    let start = page.offset();
    let end = (start + page.limit).min(total);

    let page_tasks = filtered_tasks[start..end].to_vec();
    RP::ok(total as u64, page_tasks)
}
async fn sou(State(mut state): State<AppState>, Json(m): Json<Task>) -> R<Value> {
    let mut task = m.clone();
    match state.task.add(m.cron.clone().as_str(), move || {
        let http = state.http.clone();
        let m = m.clone();
        tokio::spawn(async move {
            match http.get(&m.url).send().await {
                Ok(resp) => {
                    info!("任务执行成功 - task: {:?} response: {:?}", m, resp);
                }
                Err(e) => {
                    warn!("任务执行失败: {}", e);
                }
            };
        });
    }) {
        Ok(cron_id) => {
            let mut tasks = TASKS.lock().await;
            if task.id.is_some() {
                state.task.remove(task.cron_id.unwrap());
                task.cron_id = Some(cron_id);
                tasks.insert(task.id.unwrap(), task);
            } else {
                let next_id = tasks.keys().max().map_or(1, |&last_id| last_id + 1);
                task.id = Some(next_id);
                task.cron_id = Some(cron_id);
                tasks.insert(next_id, task);
            }
            R::ok()
        }
        Err(e) => R::err_msg(format!("添加任务失败: {}", e)),
    }
}

async fn del(State(mut state): State<AppState>, Query(id): Query<u64>) -> R<Value> {
    let mut tasks = TASKS.lock().await;
    let task = tasks
        .get(&id)
        .ok_or_else(|| R::<Value>::err_msg(format!("任务ID {} 不存在", id)))
        .unwrap();

    match state.task.remove(task.cron_id.unwrap()) {
        Ok(_) => {
            tasks.remove(&id);
            R::ok()
        }
        Err(e) => R::err_msg(format!("删除任务失败: {}", e)),
    }
}
