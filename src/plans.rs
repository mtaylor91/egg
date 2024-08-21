use serde::{Deserialize, Serialize};
use uuid::Uuid;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreatePlan {
    pub spec: PlanSpec,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub id: Uuid,
    pub spec: PlanSpec,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PlanSpec {
    Command { args: Vec<String> },
    TaskGroup { parallel: Vec<PlanSpec> },
    TaskList { serial: Vec<PlanSpec> },
}
