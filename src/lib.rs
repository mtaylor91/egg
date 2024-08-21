// File: src/lib.rs
pub mod client;
pub mod command;
mod error;
mod plans;
mod process;
mod server;
mod tasks;

pub use command::{Cli, Command};
pub use error::Error;
pub use plans::{CreatePlan, Plan};
pub use server::Server;
pub use server::serve;
pub use tasks::{CreateTask, Task, TaskStatus, TaskState};
