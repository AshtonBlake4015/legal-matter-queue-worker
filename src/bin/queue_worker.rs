use legal_matter_queue_worker::{
    infrai_queue::{InfraiError, InfraiQueue},
    legal_jobs::{process, LegalJob, LegalJobError},
};
use std::{sync::Arc, time::Duration};
use thiserror::Error;
use tokio::{sync::Semaphore, task::JoinSet, time};

const MAX_CONCURRENCY: usize = 4;
const STARTS_PER_SECOND: u64 = 2;

#[derive(Debug, Error)]
enum WorkerError {
    #[error(transparent)]
    Queue(#[from] InfraiError),
    #[error(transparent)]
    Job(#[from] LegalJobError),
    #[error("worker task failed: {0}")]
    Join(#[from] tokio::task::JoinError),
}

#[tokio::main]
async fn main() -> Result<(), WorkerError> {
    let queue = InfraiQueue::from_env()?;
    let batch = queue.consume::<LegalJob>(12, 60).await?;
    let permits = Arc::new(Semaphore::new(MAX_CONCURRENCY));
    let mut starts = time::interval(Duration::from_millis(1000 / STARTS_PER_SECOND));
    let mut tasks = JoinSet::new();

    for message in batch.items {
        starts.tick().await;
        let permit = permits.clone().acquire_owned().await.expect("semaphore remains open");
        let queue = queue.clone();
        tasks.spawn(async move {
            let outcome = process(message.payload).await?;
            queue.ack(&message.message_id).await?;
            println!("{}: {:?}", message.message_id, outcome);
            drop(permit);
            Ok::<(), WorkerError>(())
        });
    }

    while let Some(result) = tasks.join_next().await {
        result??;
    }
    Ok(())
}
