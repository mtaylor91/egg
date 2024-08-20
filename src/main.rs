use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;


pub enum Error {
    CommandFailed(std::io::Error),
    ExitFailure(std::process::ExitStatus),
    TaskNotFound(Uuid),
    TaskFailed(Uuid),
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


pub struct Server {
    tasks: Mutex<HashMap<Uuid, Arc<Mutex<ServerTask>>>>,
    verbose: bool,
}

impl Server {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
            verbose: false,
        }
    }

    pub fn run(
        &self, task_id: Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + '_>> {
        Box::pin(async move {
            // Avoid deadlocks by not holding the lock while running commands
            let steps = match self.tasks.lock().await.get(&task_id) {
                Some(task) => {
                    let mut task = task.lock().await;
                    task.status = TaskStatus::Running;
                    task.steps.clone()
                }
                None => {
                    return Err(Error::TaskNotFound(task_id));
                }
            };

            // Run each step in the task
            for step in steps.iter() {
                match step {
                    TaskStep::Command { args } => {
                        let mut command = tokio::process::Command::new(&args[0]);
                        for arg in args.iter().skip(1) {
                            command.arg(arg);
                        }

                        match command.status().await {
                            Ok(status) => {
                                if !status.success() {
                                    self.set_task_status(task_id, TaskStatus::Failure)
                                        .await;
                                    return Err(Error::ExitFailure(status));
                                }
                            }
                            Err(err) => {
                                self.set_task_status(task_id, TaskStatus::Failure)
                                    .await;
                                return Err(Error::CommandFailed(err));
                            }
                        }
                    }
                    TaskStep::Task { task } => {
                        if let Err(err) = self.run(*task).await {
                            if self.verbose {
                                eprintln!("Task {:?} failed: {:?}", task, err);
                            }
                            self.set_task_status(task_id, TaskStatus::Failure)
                                .await;
                            return Err(Error::TaskFailed(*task));
                        }
                    }
                    TaskStep::TaskGroup { tasks } => {
                        for task in tasks.iter() {
                            if let Err(err) = self.run(*task).await {
                                if self.verbose {
                                    eprintln!("Task {:?} failed: {:?}", task, err);
                                }
                                self.set_task_status(task_id, TaskStatus::Failure)
                                    .await;
                                return Err(Error::TaskFailed(*task));
                            }
                        }
                    }
                }
            }

            self.set_task_status(task_id, TaskStatus::Success).await;
            Ok(())
        })
    }

    pub async fn set_task_status(&self, task_id: Uuid, status: TaskStatus) {
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


#[derive(Debug)]
pub enum ServerError {
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
pub struct ServerTask {
    pub steps: Vec<TaskStep>,
    pub status: TaskStatus,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateTask {
    steps: Vec<TaskStep>,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    id: Uuid,
    steps: Vec<TaskStep>,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaskStep {
    Command { args: Vec<String> },
    Task { task: Uuid },
    TaskGroup { tasks: Vec<Uuid> },
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Waiting,
    Success,
    Failure,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskState {
    id: Uuid,
    status: TaskStatus,
    steps: Vec<TaskStep>,
}


async fn create_task(
    State(server): State<Arc<Server>>,
    body: Json<CreateTask>
) -> Json<Task> {
    let task = Task {
        id: Uuid::new_v4(),
        steps: body.steps.clone(),
    };

    server.tasks.lock().await.insert(
        task.id,
        Arc::new(Mutex::new(ServerTask {
            status: TaskStatus::Pending,
            steps: body.steps.clone(),
        }))
    );

    Json(task)
}


async fn list_tasks(State(server): State<Arc<Server>>) -> Json<Vec<Task>> {
    let mut tasks = vec![];

    for (id, task) in server.tasks.lock().await.iter() {
        tasks.push(Task {
            id: *id,
            steps: task.lock().await.steps.clone(),
        });
    }

    Json(tasks)
}


async fn start_task(
    State(server): State<Arc<Server>>,
    Path(task_id): Path<Uuid>
) -> Result<Json<TaskState>, ServerError> {
    let s = server.clone();
    tokio::spawn(async move {
        if let Err(err) = s.run(task_id).await {
            if s.verbose {
                eprintln!("Task failed: {:?}", err);
            }
        }
    });

    match server.tasks.lock().await.get(&task_id) {
        Some(task) => {
            let mut task = task.lock().await;
            task.status = TaskStatus::Running;
            Ok(Json(TaskState {
                id: task_id,
                status: task.status.clone(),
                steps: task.steps.clone(),
            }))
        }
        None => {
            Err(ServerError::TaskNotFound(task_id))
        }
    }
}


#[tokio::main]
async fn main() {
    let server = Arc::new(Server::new());
    let app = Router::new()
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/tasks/:task_id/start", post(start_task))
        .with_state(server);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
