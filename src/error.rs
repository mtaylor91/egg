use std::sync::Arc;
use uuid::Uuid;


#[derive(Clone)]
pub enum Error {
    NotImplemented,
    CommandFailed(Arc<std::io::Error>),
    ExitFailure(std::process::ExitStatus),
    PlanNotFound(Uuid),
    TaskNotFound(Uuid),
    TaskFailed(Uuid),
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::NotImplemented => {
                write!(f, "Not implemented")
            }
            Error::CommandFailed(err) => {
                write!(f, "Command failed: {:?}", err)
            }
            Error::ExitFailure(status) => {
                write!(f, "Command failed with exit status: {:?}", status)
            }
            Error::PlanNotFound(id) => {
                write!(f, "Plan not found: {:?}", id)
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
