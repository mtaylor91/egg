use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use uuid::Uuid;


enum Error {
    CommandFailed(std::io::Error),
    ExitFailure(std::process::ExitStatus),
    TaskNotFound(Uuid),
    TaskFailed(Uuid),
}

impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Error::CommandFailed(err) => {
                // std::io::Error does not implement Clone
                Error::CommandFailed(
                    std::io::Error::new(err.kind(), err.to_string())
                )
            }
            Error::ExitFailure(status) => {
                Error::ExitFailure(status.clone())
            }
            Error::TaskNotFound(id) => {
                Error::TaskNotFound(*id)
            }
            Error::TaskFailed(id) => {
                Error::TaskFailed(*id)
            }
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::CommandFailed(err) => {
                write!(f, "Command failed: {:?}", err)
            }
            Error::ExitFailure(status) => {
                write!(f, "Command failed with exit status: {:?}", status)
            }
            Error::TaskNotFound(id) => {
                write!(f, "Task not found: {:?}", id)
            }
            Error::TaskFailed(id) => {
                write!(f, "Task failed: {:?}", id)
            }
        }
    }
}


struct Server {
    tasks: Mutex<HashMap<Uuid, Arc<Mutex<ServerTask>>>>,
    verbose: bool,
}

impl Server {
    fn new() -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
            verbose: false,
        }
    }

    async fn set_task_status(&self, task_id: Uuid, status: TaskStatus) {
        match self.tasks.lock().await.get(&task_id) {
            Some(task) => {
                let mut task = task.lock().await;
                task.status = status;
            },
            None => {
                return;
            }
        }
    }
}


enum ServerError {
    TaskNotFound(Uuid),
}

impl axum::response::IntoResponse for ServerError {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        match self {
            ServerError::TaskNotFound(id) => {
                (
                    axum::http::StatusCode::NOT_FOUND,
                    format!("Task not found: {:?}", id)
                ).into_response()
            }
        }
    }
}


#[derive(Debug)]
struct ServerTask {
    spec: TaskSpec,
    status: TaskStatus,
    finished: Arc<Notify>,
    error: Option<Error>,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
struct CreateTask {
    spec: TaskSpec,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
struct Task {
    id: Uuid,
    spec: TaskSpec,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum TaskSpec {
    Command { args: Vec<String> },
    TaskGroup { parallel: Vec<Uuid> },
    TaskList { serial: Vec<TaskSpec> },
}


#[derive(Clone, Debug, Serialize, Deserialize)]
enum TaskStatus {
    Pending,
    Running,
    Waiting,
    Success,
    Failure,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
struct TaskState {
    id: Uuid,
    spec: TaskSpec,
    status: TaskStatus,
}


async fn create_task_handler(
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
            finished: Arc::new(Notify::new()),
            error: None,
        }))
    );

    Json(task)
}


async fn list_tasks_handler(State(server): State<Arc<Server>>) -> Json<Vec<Task>> {
    let mut tasks = vec![];

    for (id, task) in server.tasks.lock().await.iter() {
        tasks.push(Task {
            id: *id,
            spec: task.lock().await.spec.clone(),
        });
    }

    Json(tasks)
}


async fn start_task_handler(
    State(server): State<Arc<Server>>,
    Path(task_id): Path<Uuid>
) -> Result<Json<TaskState>, ServerError> {
    let state = start_task(server, task_id).await?;
    Ok(Json(state))
}


#[tokio::main]
async fn main() {
    let server = Arc::new(Server::new());
    let app = Router::new()
        .route("/tasks", get(list_tasks_handler).post(create_task_handler))
        .route("/tasks/:task_id/start", post(start_task_handler))
        .with_state(server);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}


fn start_task(
    server: Arc<Server>,
    task_id: Uuid
) -> Pin<Box<dyn Future<Output = Result<TaskState, ServerError>> + Send>> {
    Box::pin(async move {
        if server.verbose {
            eprintln!("Starting task: {:?}", task_id);
        }

        let task = match server.tasks.lock().await.get(&task_id) {
            Some(task) => task.clone(),
            None => {
                return Err(ServerError::TaskNotFound(task_id));
            }
        };

        let mut task = task.lock().await;
        match task.spec {
            TaskSpec::Command { .. } => {
                task.status = TaskStatus::Running;
            }
            TaskSpec::TaskGroup { .. } => {
                task.status = TaskStatus::Waiting;
            }
            TaskSpec::TaskList { .. } => {
                task.status = TaskStatus::Waiting;
            }
        }

        tokio::spawn(async move {
            run_task(server, task_id).await;
        });

        Ok(TaskState {
            id: task_id,
            spec: task.spec.clone(),
            status: task.status.clone(),
        })
    })
}

async fn run_task(
    server: Arc<Server>,
    task_id: Uuid
) {
    let task = match server.tasks.lock().await.get(&task_id) {
        Some(task) => task.clone(),
        None => {
            return;
        }
    };

    let spec = task.lock().await.spec.clone();
    match spec {
        TaskSpec::Command { ref args } => {
            let result = tokio::process::Command::new(&args[0])
                .args(&args[1..])
                .spawn();
            match result {
                Ok(mut process) => {
                    let result = process.wait().await;
                    match result {
                        Ok(status) => {
                            if status.success() {
                                finish_task(server, task_id).await;
                            } else {
                                fail_task(server, task_id, Error::ExitFailure(status))
                                    .await;
                            }
                        }
                        Err(err) => {
                            fail_task(server, task_id,
                                Error::CommandFailed(err)).await;
                        }
                    }
                }
                Err(err) => {
                    let mut task = task.lock().await;
                    task.error = Some(Error::CommandFailed(err));
                    task.status = TaskStatus::Failure;
                }
            }
        }
        TaskSpec::TaskGroup { ref parallel } => {
            let mut tasks = vec![];
            for child_id in parallel {
                let server = server.clone();
                tasks.push(async move {
                    match start_task(server.clone(), *child_id).await {
                        Ok(_) => {
                            if let Err(err) = wait_task(server.clone(), *child_id).await {
                                fail_task(server, task_id, err).await;
                            }
                        }
                        Err(ServerError::TaskNotFound(_)) => {
                            fail_task(server, task_id,
                                Error::TaskNotFound(*child_id)).await;
                        }
                    }
                });
            }

            for task in tasks {
                let _ = task.await;
            }

            finish_task(server, task_id).await;
        }
        _ => {
            eprintln!("Not implemented");
            let mut task = task.lock().await;
            task.status = TaskStatus::Failure;
        }
    }
}


async fn finish_task(
    server: Arc<Server>,
    task_id: Uuid
) {
    if server.verbose {
        eprintln!("Finished task: {:?}", task_id);
    }

    server.set_task_status(task_id, TaskStatus::Success).await;
    if let Some(task) = server.tasks.lock().await.get(&task_id) {
        let task = task.lock().await;
        task.finished.notify_waiters();
    }
}


async fn fail_task(
    server: Arc<Server>,
    task_id: Uuid,
    error: Error
) {
    if server.verbose {
        eprintln!("Failed task {}: {:?}", task_id, error);
    }

    server.set_task_status(task_id, TaskStatus::Failure).await;
    if let Some(task) = server.tasks.lock().await.get(&task_id) {
        let mut task = task.lock().await;
        task.error = Some(error);
        task.finished.notify_waiters();
    }
}


async fn wait_task(
    server: Arc<Server>,
    task_id: Uuid
) -> Result<(), Error> {
    let finished = match server.tasks.lock().await.get(&task_id) {
        Some(task) => task.lock().await.finished.clone(),
        None => {
            return Err(Error::TaskNotFound(task_id));
        }
    };

    finished.notified().await;
    match server.tasks.lock().await.get(&task_id) {
        Some(task) => {
            let task = task.lock().await;
            match task.status {
                TaskStatus::Success => {
                    Ok(())
                }
                TaskStatus::Failure => {
                    Err(task.error.clone().unwrap())
                }
                _ => {
                    eprintln!("Task not finished: {:?}", task_id);
                    Err(Error::TaskFailed(task_id))
                }
            }
        }
        None => {
            Err(Error::TaskNotFound(task_id))
        }
    }
}
