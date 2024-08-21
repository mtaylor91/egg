use futures::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use uuid::Uuid;

use crate::error::Error;
use crate::plans::PlanSpec;
use crate::server::{Server, ServerTask};
use crate::tasks::{Task, TaskPlan, TaskSpec, TaskStatus};


pub fn task(
    server: Arc<Server>,
    plan: TaskPlan,
    spec: PlanSpec,
) -> Pin<Box<dyn Future<Output = Result<Task, Error>> + Send>> {
    Box::pin(async move {
        match spec {
            PlanSpec::Command { args } => {
                let task = Task {
                    id: Uuid::new_v4(),
                    plan: Some(plan.clone()),
                    spec: TaskSpec::Command { args },
                    status: TaskStatus::Pending,
                };

                server.tasks.lock().await.insert(
                    task.id,
                    Arc::new(Mutex::new(ServerTask {
                        plan: Some(plan.clone()),
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
                    let child = task(server.clone(), plan.clone(), child_spec).await?;
                    tasks.push(child.id);
                }

                let task = Task {
                    id: Uuid::new_v4(),
                    plan: Some(plan.clone()),
                    spec: TaskSpec::TaskGroup { parallel: tasks },
                    status: TaskStatus::Pending,
                };

                server.tasks.lock().await.insert(
                    task.id,
                    Arc::new(Mutex::new(ServerTask {
                        plan: Some(plan.clone()),
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
                    let child = task(server.clone(), plan.clone(), child_spec).await?;
                    tasks.push(child.id);
                }

                let task = Task {
                    id: Uuid::new_v4(),
                    plan: Some(plan.clone()),
                    spec: TaskSpec::TaskList { serial: tasks },
                    status: TaskStatus::Pending,
                };

                server.tasks.lock().await.insert(
                    task.id,
                    Arc::new(Mutex::new(ServerTask {
                        plan: Some(plan.clone()),
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
