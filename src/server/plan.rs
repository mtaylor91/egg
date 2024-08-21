use futures::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use uuid::Uuid;

use crate::error::Error;
use crate::plans::PlanSpec;
use crate::server::{Server, ServerTask};
use crate::tasks::{Task, TaskSpec, TaskStatus};


pub fn task(
    server: Arc<Server>,
    spec: PlanSpec,
) -> Pin<Box<dyn Future<Output = Result<Task, Error>> + Send>> {
    Box::pin(async move {
        match spec {
            PlanSpec::Command { args } => {
                let task = Task {
                    id: Uuid::new_v4(),
                    spec: TaskSpec::Command { args },
                    status: TaskStatus::Pending,
                };

                server.tasks.lock().await.insert(
                    task.id,
                    Arc::new(Mutex::new(ServerTask {
                        spec: task.spec.clone(),
                        status: TaskStatus::Pending,
                        running: None,
                        finished: Arc::new(Notify::new()),
                        error: None,
                    }))
                );

                Ok(task)
            }
            PlanSpec::TaskGroup { parallel } => {
                let mut tasks = Vec::new();
                for child_spec in parallel {
                    let child_task = task(server.clone(), child_spec).await?;
                    tasks.push(child_task.id);
                }

                let task = Task {
                    id: Uuid::new_v4(),
                    spec: TaskSpec::TaskGroup { parallel: tasks },
                    status: TaskStatus::Pending,
                };

                server.tasks.lock().await.insert(
                    task.id,
                    Arc::new(Mutex::new(ServerTask {
                        spec: task.spec.clone(),
                        status: TaskStatus::Pending,
                        running: None,
                        finished: Arc::new(Notify::new()),
                        error: None,
                    }))
                );

                Ok(task)
            }
            PlanSpec::TaskList { serial } => {
                let mut tasks = Vec::new();
                for child_spec in serial {
                    let child_task = task(server.clone(), child_spec).await?;
                    tasks.push(child_task.id);
                }

                let task = Task {
                    id: Uuid::new_v4(),
                    spec: TaskSpec::TaskList { serial: tasks },
                    status: TaskStatus::Pending,
                };

                server.tasks.lock().await.insert(
                    task.id,
                    Arc::new(Mutex::new(ServerTask {
                        spec: task.spec.clone(),
                        status: TaskStatus::Pending,
                        running: None,
                        finished: Arc::new(Notify::new()),
                        error: None,
                    }))
                );

                Ok(task)
            }
        }
    })
}
