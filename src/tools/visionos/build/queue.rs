use std::{collections::VecDeque, sync::Arc};

use chrono::{DateTime, Utc};
use tokio::sync::{Mutex, Notify};
use uuid::Uuid;

/// Ticket that identifies a build job.
#[derive(Debug, Clone)]
pub struct JobTicket {
    pub job_id: Uuid,
    pub enqueued_at: DateTime<Utc>,
}

/// Single job queue shared by the visionOS build tools.
#[derive(Clone)]
pub struct VisionOsJobQueue {
    inner: Arc<VisionOsJobQueueInner>,
}

struct VisionOsJobQueueInner {
    queue: Mutex<VecDeque<JobTicket>>,
    notify: Notify,
}

impl Default for VisionOsJobQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl VisionOsJobQueue {
    /// Create an empty job queue.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(VisionOsJobQueueInner {
                queue: Mutex::new(VecDeque::new()),
                notify: Notify::new(),
            }),
        }
    }

    /// Enqueue a job and wait until it reaches the front.
    pub async fn wait_for_turn(&self, job_id: Uuid) -> JobTicket {
        let ticket = JobTicket {
            job_id,
            enqueued_at: Utc::now(),
        };
        {
            let mut queue = self.inner.queue.lock().await;
            queue.push_back(ticket.clone());
        }

        loop {
            {
                let queue = self.inner.queue.lock().await;
                if matches!(queue.front(), Some(front) if front.job_id == job_id) {
                    break;
                }
            }
            self.inner.notify.notified().await;
        }

        ticket
    }

    /// Notify completion and wake the next job.
    pub async fn finish_job(&self, job_id: Uuid) {
        {
            let mut queue = self.inner.queue.lock().await;
            if matches!(queue.front(), Some(front) if front.job_id == job_id) {
                queue.pop_front();
            }
        }
        self.inner.notify.notify_waiters();
    }

    /// Return the number of pending jobs (used for telemetry).
    pub async fn pending_jobs(&self) -> usize {
        let queue = self.inner.queue.lock().await;
        queue.len()
    }
}
