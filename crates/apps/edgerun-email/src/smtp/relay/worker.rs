//! Delivery worker — background process that drains the mail queue and
//! attempts delivery with exponential backoff.

use crate::prelude::*;
use std::sync::Arc;
use std::time::Duration;

use crate::rt::CancellationToken;

use crate::smtp::relay::bounce::{send_bounce, BounceConfig};
use crate::smtp::relay::queue::MailIndex;
use crate::smtp::relay::relay::OutboundRelay;
use crate::smtp::types::MailEnvelope;

/// Configuration for the delivery worker.
#[derive(Clone)]
pub struct DeliveryWorkerConfig {
    /// Maximum number of messages to process per batch.
    pub batch_size: usize,
    /// Minimum retry interval (base of exponential backoff).
    pub min_retry_interval: Duration,
    /// Maximum retry interval (cap of exponential backoff).
    pub max_retry_interval: Duration,
    /// Maximum number of retries before bouncing.
    pub max_retries: i32,
    /// Poll interval when queue is empty.
    pub poll_interval: Duration,
    /// Bounce sender configuration.
    pub bounce_config: BounceConfig,
}

impl Default for DeliveryWorkerConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            min_retry_interval: Duration::from_secs(300), // 5 minutes
            max_retry_interval: Duration::from_secs(86400), // 24 hours
            max_retries: 8,
            poll_interval: Duration::from_secs(10),
            bounce_config: BounceConfig::default(),
        }
    }
}

/// Background worker that processes the outbound mail queue.
pub struct DeliveryWorker {
    config: DeliveryWorkerConfig,
    relay: OutboundRelay,
    queue: Arc<MailIndex>,
}

impl DeliveryWorker {
    pub fn new(config: DeliveryWorkerConfig, relay: OutboundRelay, queue: Arc<MailIndex>) -> Self {
        Self {
            config,
            relay,
            queue,
        }
    }

    /// Run the delivery worker loop until shutdown is cancelled.
    pub async fn run(&self, shutdown: CancellationToken) {
        edgerun_log::info!("edgerun-smtp: delivery worker started");

        loop {
            if shutdown.is_cancelled() {
                break;
            }

            let now = current_time_secs();

            // Dequeue messages due for delivery
            let messages = self.queue.dequeue_due(now, self.config.batch_size).await;
            if messages.is_empty() {
                crate::rt::sleep(self.config.poll_interval).await;
                continue;
            }

            for msg in &messages {
                if shutdown.is_cancelled() {
                    break;
                }

                self.process_message(msg).await;
            }
        }

        edgerun_log::info!("edgerun-smtp: delivery worker stopped");
    }

    /// Process a single message — attempt delivery to each recipient.
    async fn process_message(&self, msg: &crate::smtp::relay::queue::MailMessageRecord) {
        // Mark as sending
        if let Err(e) = self.queue.mark_sending(&msg.message_id).await {
            edgerun_log::error!("edgerun-smtp: failed to mark message as sending: {}", e);
            return;
        }

        // Build envelope from queued data
        let mut envelope = MailEnvelope::new(msg.envelope_sender.clone());
        envelope.data = msg.data.clone();

        let mut any_failed = false;

        for recipient in &msg.recipients {
            // Skip already delivered recipients
            if self
                .queue
                .is_recipient_delivered(&msg.message_id, recipient)
                .await
            {
                continue;
            }

            edgerun_log::info!(
                "edgerun-smtp: delivering message {} to {} (attempt {})",
                msg.message_id,
                recipient,
                msg.retry_count + 1
            );

            match self.relay.deliver_to_recipient(&envelope, recipient).await {
                Ok(remote_mta) => {
                    edgerun_log::info!(
                        "edgerun-smtp: delivered message {} to {} via {}",
                        msg.message_id,
                        recipient,
                        remote_mta
                    );
                    if let Err(e) = self
                        .queue
                        .mark_delivered(&msg.message_id, recipient, Some(&remote_mta))
                        .await
                    {
                        edgerun_log::error!("edgerun-smtp: failed to mark delivered: {}", e);
                    }
                }
                Err(reason) => {
                    edgerun_log::warn!(
                        "edgerun-smtp: delivery failed for message {} to {}: {}",
                        msg.message_id,
                        recipient,
                        reason
                    );
                    any_failed = true;
                    if let Err(e) = self
                        .queue
                        .mark_failed(&msg.message_id, recipient, &reason, None)
                        .await
                    {
                        edgerun_log::error!("edgerun-smtp: failed to mark failed: {}", e);
                    }
                }
            }
        }

        // Determine next action
        if !any_failed {
            edgerun_log::info!("edgerun-smtp: message {} fully delivered", msg.message_id);
        } else {
            let next_retry = msg.retry_count + 1;
            if next_retry >= self.config.max_retries {
                // Max retries exceeded — send DSN bounce
                edgerun_log::error!(
                    "edgerun-smtp: message {} exceeded max retries ({}), bouncing",
                    msg.message_id,
                    self.config.max_retries
                );

                // Get failed recipients
                let failed = self.queue.get_failed_recipients(&msg.message_id).await;

                // Send bounce to original sender
                match send_bounce(
                    &msg.envelope_sender,
                    &failed,
                    &msg.data,
                    &self.config.bounce_config,
                )
                .await
                {
                    Ok(()) => {
                        edgerun_log::info!(
                            "edgerun-smtp: bounce sent to {} for message {}",
                            msg.envelope_sender,
                            msg.message_id,
                        );
                    }
                    Err(e) => {
                        edgerun_log::error!(
                            "edgerun-smtp: failed to send bounce for message {}: {}",
                            msg.message_id,
                            e,
                        );
                    }
                }

                if let Err(e) = self.queue.mark_bounced(&msg.message_id).await {
                    edgerun_log::error!("edgerun-smtp: failed to mark bounced: {}", e);
                }
            } else {
                let delay = self.calculate_backoff(next_retry);
                let next_retry_time = current_time_secs() + delay.as_secs() as i64;
                edgerun_log::info!(
                    "edgerun-smtp: scheduling retry for message {} in {}s (attempt {}/{})",
                    msg.message_id,
                    delay.as_secs(),
                    next_retry,
                    self.config.max_retries
                );
                if let Err(e) = self
                    .queue
                    .schedule_retry(&msg.message_id, next_retry, next_retry_time)
                    .await
                {
                    edgerun_log::error!("edgerun-smtp: failed to schedule retry: {}", e);
                }
            }
        }
    }

    /// Calculate exponential backoff with jitter.
    fn calculate_backoff(&self, attempt: i32) -> Duration {
        let base = self.config.min_retry_interval.as_secs();
        let max = self.config.max_retry_interval.as_secs();

        // Exponential: base * 2^(attempt-1), capped at max
        let delay = base.saturating_mul(1 << (attempt as u32).min(20));
        let delay = delay.min(max);

        Duration::from_secs(delay)
    }
}

fn current_time_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
