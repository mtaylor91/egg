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
pub struct Process {
    inner: Mutex<ProcessState>,
    output: Notify,
    exited: Notify,
}

impl Process {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(ProcessState {
                output: vec![],
                status: None,
            }),
            output: Notify::new(),
            exited: Notify::new(),
        }
    }

    pub async fn run(
        self: Arc<Self>,
        args: &[String],
        verbose: bool
    ) -> Result<(), Error> {

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

        let self_clone = self.clone();
        tokio::spawn(async move {
            while let Some(line) = stdout.next_line().await.unwrap() {
                let mut inner = self_clone.inner.lock().await;
                inner.output.push(Output::Stdout(line.clone()));
                self_clone.output.notify_waiters();
                if verbose {
                    eprintln!("{}", line);
                }
            }
        });

        let self_clone = self.clone();
        tokio::spawn(async move {
            while let Some(line) = stderr.next_line().await.unwrap() {
                let mut inner = self_clone.inner.lock().await;
                inner.output.push(Output::Stderr(line.clone()));
                self_clone.output.notify_waiters();
                if verbose {
                    eprintln!("{}", line);
                }
            }
        });

        let status = process.wait().await
            .map_err(|err| Error::CommandFailed(Arc::new(err)))?;
        let mut inner = self.inner.lock().await;
        inner.status = Some(status);
        self.exited.notify_waiters();
        Ok(())
    }
}


#[derive(Debug)]
pub struct OutputStream {
    inner: Arc<Process>,
    index: usize,
}

impl OutputStream {
    pub fn new(inner: Arc<Process>) -> Self {
        Self { inner, index: 0 }
    }
}

impl Stream for OutputStream {
    type Item = Output;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let inner = match this.inner.inner.try_lock() {
            Ok(inner) => inner,
            Err(_) => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };

        if let Some(output) = inner.output.get(this.index) {
            this.index += 1;
            Poll::Ready(Some(output.clone()))
        } else if inner.status.is_some() {
            Poll::Ready(None)
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Output {
    Stdout(String),
    Stderr(String),
}


#[derive(Debug)]
struct ProcessState {
    output: Vec<Output>,
    status: Option<ExitStatus>,
}
