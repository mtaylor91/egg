use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::process::ExitStatus;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::{Mutex, Notify};
use tokio::io::AsyncBufReadExt;

use crate::error::Error;


#[derive(Debug)]
pub struct Command {
    inner: Mutex<CommandState>,
    output: Notify,
    exited: Notify,
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
                            inner.output.push(Output::Stdout(line));
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
                            inner.output.push(Output::Stderr(line));
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

        let status = process.wait().await
            .map_err(|err| Error::CommandFailed(Arc::new(err)))?;
        let mut inner = self.inner.lock().await;
        inner.status = Some(status);
        self.exited.notify_waiters();
        Ok(())
    }
}


#[derive(Debug)]
pub struct CommandStream {
    inner: Arc<Command>,
}

impl CommandStream {
    pub fn new(inner: Arc<Command>) -> Self {
        Self { inner }
    }
}

impl Stream for CommandStream {
    type Item = Output;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let mut inner = match this.inner.inner.try_lock() {
            Ok(inner) => inner,
            Err(_) => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };

        if let Some(output) = inner.output.pop() {
            Poll::Ready(Some(output))
        } else if inner.status.is_some() {
            Poll::Ready(None)
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub enum Output {
    Stdout(String),
    Stderr(String),
}


#[derive(Debug)]
struct CommandState {
    output: Vec<Output>,
    status: Option<ExitStatus>,
}
