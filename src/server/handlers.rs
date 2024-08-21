use axum::{extract::{Path, State}, response::IntoResponse, Json};
use axum_streams::StreamBodyAs;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use uuid::Uuid;

use crate::command::CommandStream;
use crate::error::Error;
use crate::plans::{CreatePlan, Plan};
use crate::server::{Server, ServerError, ServerTask};
use crate::tasks::{CreateTask, Task, TaskStatus, TaskState};


pub async fn create_plan(
    State(server): State<Arc<Server>>,
    body: Json<CreatePlan>
) -> Json<Plan> {
    let plan = Plan {
        id: Uuid::new_v4(),
        spec: body.spec.clone(),
    };

    server.plans.lock().await.insert(
        plan.id,
        Arc::new(Mutex::new(plan.clone()))
    );

    Json(plan)
}


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


pub async fn get_plan(
    State(server): State<Arc<Server>>,
    Path(plan_id): Path<Uuid>
) -> Result<Json<Plan>, ServerError> {
    match server.plans.lock().await.get(&plan_id) {
        Some(plan) => Ok(Json(Plan {
            id: plan_id,
            spec: plan.lock().await.spec.clone(),
        })),
        None => Err(ServerError::TaskNotFound(plan_id)),
    }
}


pub async fn list_plans(State(server): State<Arc<Server>>) -> Json<Vec<Plan>> {
    let mut plans = vec![];

    for (id, plan) in server.plans.lock().await.iter() {
        plans.push(Plan {
            id: *id,
            spec: plan.lock().await.spec.clone(),
        });
    }

    Json(plans)
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


pub async fn plan(
    State(server): State<Arc<Server>>,
    Path(plan_id): Path<Uuid>
) -> Result<Json<Task>, ServerError> {
    let plan_spec = match server.plans.lock().await.get(&plan_id) {
        Some(plan) => plan.lock().await.spec.clone(),
        None => {
            return Err(ServerError::PlanNotFound(plan_id));
        }
    };

    match crate::server::plan::task(server, plan_spec).await {
        Ok(task) => Ok(Json(task)),
        Err(Error::PlanNotFound(id)) => Err(ServerError::PlanNotFound(id)),
        Err(_) => Err(ServerError::InternalServerError),
    }
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
