use crate::plans::{CreatePlan, Plan};
use crate::tasks::{Task, TaskState};


#[derive(Clone, Debug)]
pub struct Client {
    reqwest: reqwest::Client,
    server: String,
}

impl Client {
    pub fn new(server: String) -> Self {
        Self {
            reqwest: reqwest::Client::new(),
            server,
        }
    }

    pub async fn create_plan(&self, plan: &CreatePlan) -> Result<Plan, reqwest::Error> {
        let response = self.reqwest
            .post(&format!("{}/plans", self.server))
            .json(plan)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(response.error_for_status().unwrap_err());
        }

        Ok(response.json().await?)
    }

    pub async fn get_task(&self, task_id: uuid::Uuid) -> Result<Task, reqwest::Error> {
        let response = self.reqwest
            .get(&format!("{}/tasks/{}", self.server, task_id))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(response.error_for_status().unwrap_err());
        }

        Ok(response.json().await?)
    }

    pub async fn plan(&self, plan_id: uuid::Uuid) -> Result<Task, reqwest::Error> {
        let response = self.reqwest
            .get(&format!("{}/plan/{}", self.server, plan_id))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(response.error_for_status().unwrap_err());
        }

        Ok(response.json().await?)
    }

    pub async fn start_task(
        &self,
        task_id: uuid::Uuid
    ) -> Result<TaskState, reqwest::Error> {
        let response = self.reqwest
            .post(&format!("{}/tasks/{}/start", self.server, task_id))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(response.error_for_status().unwrap_err());
        }

        Ok(response.json().await?)
    }
}
