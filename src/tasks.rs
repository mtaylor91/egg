use serde::{Deserialize, Serialize};
use uuid::Uuid;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateTask {
    pub spec: TaskSpec,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub plan: Option<TaskPlan>,
    pub spec: TaskSpec,
    pub status: TaskStatus,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskPlan {
    pub id: Uuid,
    pub version: u64,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaskSpec {
    Command { args: Vec<String> },
    TaskGroup { parallel: Vec<Uuid> },
    TaskList { serial: Vec<Uuid> },
}


#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Waiting,
    Success,
    Failure,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskState {
    pub id: Uuid,
    pub spec: TaskSpec,
    pub status: TaskStatus,
}
