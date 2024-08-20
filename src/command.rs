use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tokio::io::AsyncBufReadExt;

use crate::error::Error;


#[derive(Debug)]
pub struct Command {
    inner: Mutex<CommandState>,
    output: Notify,
    exited: Notify,
}


#[derive(Debug)]
struct CommandState {
    output: Vec<u8>,
    status: Option<std::process::ExitStatus>,
}


impl Command {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(CommandState {
                output: vec![],
                status: None,
            }),
            output: Notify::new(),
            exited: Notify::new(),
        }
    }

    pub async fn run(&self, args: &[String]) -> Result<(), Error> {
        let mut process = tokio::process::Command::new(&args[0])
            .args(&args[1..])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|err| Error::CommandFailed(Arc::new(err)))?;

        let stdout = process.stdout.take().expect("failed to get stdout");
        let stderr = process.stderr.take().expect("failed to get stderr");

        let mut stdout = tokio::io::BufReader::new(stdout).lines();
        let mut stderr = tokio::io::BufReader::new(stderr).lines();

        loop {
            tokio::select! {
                line = stdout.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            let mut inner = self.inner.lock().await;
                            inner.output.extend_from_slice(line.as_bytes());
                            inner.output.push(b'\n');
                            self.output.notify_waiters();
                        }
                        Ok(None) => {
                            break;
                        }
                        Err(err) => {
                            return Err(Error::CommandFailed(Arc::new(err)));
                        }
                    }
                }
                line = stderr.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            let mut inner = self.inner.lock().await;
                            inner.output.extend_from_slice(line.as_bytes());
                            inner.output.push(b'\n');
                            self.output.notify_waiters();
                        }
                        Ok(None) => {
                            break;
                        }
                        Err(err) => {
                            return Err(Error::CommandFailed(Arc::new(err)));
                        }
                    }
                }
            }
        }

        let mut inner = self.inner.lock().await;
        let status = process.wait().await.expect("failed to wait for command");
        inner.status = Some(status);
        self.exited.notify_waiters();
        Ok(())
    }
}
