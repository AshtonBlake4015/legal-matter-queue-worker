use legal_matter_queue_worker::{infrai_queue::InfraiQueue, legal_jobs::LegalJob};

#[tokio::main]
async fn main() -> Result<(), legal_matter_queue_worker::infrai_queue::InfraiError> {
    let job = LegalJob::DeadlineFollowUp {
        matter_id: "MAT-1042".into(),
        deadline: "2026-08-18T09:00:00Z".into(),
        hours_remaining: 36,
    };
    let queue = InfraiQueue::from_env()?;
    queue.publish(&job, "deadline-follow-up:MAT-1042:2026-08-18").await?;
    println!("queued deadline follow-up for MAT-1042");
    Ok(())
}
