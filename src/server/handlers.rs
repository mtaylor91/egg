use axum::{extract::{Path, State}, response::IntoResponse, Json};
use axum_streams::StreamBodyAs;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use uuid::Uuid;

use crate::command::CommandStream;
use crate::server::{Server, ServerError, ServerTask};
use crate::tasks::{CreateTask, Task, TaskStatus, TaskState};


pub async fn create_task(
    State(server): State<Arc<Server>>,
    body: Json<CreateTask>
) -> Json<Task> {
    let task = Task {
        id: Uuid::new_v4(),
        spec: body.spec.clone(),
    };

    server.tasks.lock().await.insert(
        task.id,
        Arc::new(Mutex::new(ServerTask {
            spec: body.spec.clone(),
            status: TaskStatus::Pending,
            running: None,
            finished: Arc::new(Notify::new()),
            error: None,
        }))
    );

    Json(task)
}


pub async fn list_tasks(State(server): State<Arc<Server>>) -> Json<Vec<Task>> {
    let mut tasks = vec![];

    for (id, task) in server.tasks.lock().await.iter() {
        tasks.push(Task {
            id: *id,
            spec: task.lock().await.spec.clone(),
        });
    }

    Json(tasks)
}


pub async fn start_task(
    State(server): State<Arc<Server>>,
    Path(task_id): Path<Uuid>
) -> Result<Json<TaskState>, ServerError> {
    Ok(Json(crate::server::run::start_task(server, task_id).await?))
}


pub async fn task_output_stream(
    State(server): State<Arc<Server>>,
    Path(task_id): Path<Uuid>
) -> Result<impl IntoResponse, ServerError> {
    let cmd = match server.tasks.lock().await.get(&task_id) {
        Some(task) => {
            match task.lock().await.running {
                Some(ref cmd) => cmd.clone(),
                None => {
                    return Err(ServerError::TaskNotFound(task_id));
                }
            }
        }
        None => {
            return Err(ServerError::TaskNotFound(task_id));
        }
    };

    Ok(StreamBodyAs::json_nl(CommandStream::new(cmd.clone())))
}
