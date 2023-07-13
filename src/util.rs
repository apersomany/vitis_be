use std::{future::Future, pin::Pin, sync::OnceLock, time::UNIX_EPOCH};

use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use tokio::{
    runtime::Handle,
    spawn,
    sync::{
        oneshot::{channel, Sender},
        Mutex, MutexGuard,
    },
    task::JoinHandle,
};

pub fn iso(str: &str) -> Result<i64> {
    let timestamp = str
        .replace(".000Z", "")
        .parse::<NaiveDateTime>()?
        .timestamp();
    Ok(timestamp)
}

pub fn now() -> i64 {
    UNIX_EPOCH.elapsed().unwrap().as_secs() as i64
}

pub fn spawn_solo<T: Send + 'static>(
    future: impl Future<Output = T> + Send + 'static,
) -> JoinHandle<T> {
    spawn(async move {
        let (completion_sender, completion_recver) = channel();
        loop {
            let submission_sender = lock_submission_queue().await.pop();
            if let Some(submission_sender) = submission_sender {
                let _ = submission_sender.send(Box::pin(async move {
                    let _ = completion_sender.send(future.await);
                }));
                return completion_recver.await.unwrap();
            } else {
                let handle = Handle::current();
                std::thread::spawn(move || loop {
                    handle.block_on(async {
                        let (submission_sender, submission_recver) = channel();
                        let mut submission_queue = lock_submission_queue().await;
                        submission_queue.push(submission_sender);
                        drop(submission_queue);
                        submission_recver.await.unwrap().await;
                    })
                });
            };
        }
    })
}

async fn lock_submission_queue() -> MutexGuard<'static, SubmissionQueue> {
    SUBMISSION_QUEUE
        .get_or_init(|| Mutex::default())
        .lock()
        .await
}

static SUBMISSION_QUEUE: OnceLock<Mutex<SubmissionQueue>> = OnceLock::new();

type SubmissionQueue = Vec<Sender<Pin<Box<dyn Future<Output = ()> + Send>>>>;

pub fn get_param(url: &str, key: &str) -> Result<String> {
    fn inner(url: &str, key: &str) -> Option<String> {
        let val = url
            .split(&format!("{key}="))
            .nth(1)?
            .split("&")
            .nth(0)?
            .to_string();
        Some(val)
    }
    if let Some(val) = inner(url, key) {
        Ok(val)
    } else {
        Err(anyhow!("could not get param {key} in {url}"))?
    }
}
