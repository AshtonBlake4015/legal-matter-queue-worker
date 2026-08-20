use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LegalJob {
    MatterIntake {
        matter_id: String,
        client_name: String,
        conflict_cleared: bool,
    },
    SignedDocumentDelivery {
        matter_id: String,
        document_id: String,
        recipient: String,
    },
    DeadlineFollowUp {
        matter_id: String,
        deadline: String,
        hours_remaining: u32,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum JobOutcome {
    IntakeOpened { matter_id: String },
    IntakeHeld { matter_id: String },
    DeliveryRecorded { document_id: String },
    ReminderQueued { matter_id: String, urgent: bool },
}

#[derive(Debug, Error)]
pub enum LegalJobError {
    #[error("matter id must not be empty")]
    MissingMatter,
    #[error("document id and recipient must not be empty")]
    InvalidDelivery,
    #[error("deadline must not be empty")]
    InvalidDeadline,
}

pub async fn process(job: LegalJob) -> Result<JobOutcome, LegalJobError> {
    match job {
        LegalJob::MatterIntake { matter_id, conflict_cleared, .. } => {
            require_matter(&matter_id)?;
            if conflict_cleared {
                Ok(JobOutcome::IntakeOpened { matter_id })
            } else {
                Ok(JobOutcome::IntakeHeld { matter_id })
            }
        }
        LegalJob::SignedDocumentDelivery { matter_id, document_id, recipient } => {
            require_matter(&matter_id)?;
            if document_id.is_empty() || recipient.is_empty() {
                return Err(LegalJobError::InvalidDelivery);
            }
            Ok(JobOutcome::DeliveryRecorded { document_id })
        }
        LegalJob::DeadlineFollowUp { matter_id, deadline, hours_remaining } => {
            require_matter(&matter_id)?;
            if deadline.is_empty() {
                return Err(LegalJobError::InvalidDeadline);
            }
            Ok(JobOutcome::ReminderQueued {
                matter_id,
                urgent: hours_remaining <= 48,
            })
        }
    }
}

fn require_matter(matter_id: &str) -> Result<(), LegalJobError> {
    if matter_id.is_empty() { Err(LegalJobError::MissingMatter) } else { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn escalates_only_deadlines_inside_the_48_hour_window() {
        let near = process(LegalJob::DeadlineFollowUp {
            matter_id: "MAT-1042".into(),
            deadline: "2026-08-18T09:00:00Z".into(),
            hours_remaining: 36,
        })
        .await
        .unwrap();
        let later = process(LegalJob::DeadlineFollowUp {
            matter_id: "MAT-1043".into(),
            deadline: "2026-08-22T09:00:00Z".into(),
            hours_remaining: 120,
        })
        .await
        .unwrap();

        assert_eq!(near, JobOutcome::ReminderQueued { matter_id: "MAT-1042".into(), urgent: true });
        assert_eq!(later, JobOutcome::ReminderQueued { matter_id: "MAT-1043".into(), urgent: false });
    }
}
