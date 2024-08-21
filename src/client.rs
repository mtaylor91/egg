use crate::plans::{CreatePlan, Plan};


#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::Reqwest(err)
    }
}


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

    pub async fn create_plan(&self, plan: &CreatePlan) -> Result<Plan, Error> {
        let response = self.reqwest
            .post(&format!("{}/plans", self.server))
            .json(plan)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Reqwest(response.error_for_status().unwrap_err()));
        }

        Ok(response.json().await?)
    }
}
