//! Mail queue management CLI.
//!
//! # Usage
//! ```text
//! edgerun-mailq list <data-root>                    — List all queued messages
//! edgerun-mailq show <data-root> <message-id>       — Show message details
//! edgerun-mailq retry <data-root> <message-id>      — Force retry a message
//! edgerun-mailq retry-all <data-root>               — Retry all pending messages
//! edgerun-mailq delete <data-root> <message-id>     — Delete a message from queue
//! edgerun-mailq stats <data-root>                   — Queue statistics
//! ```

use std::path::PathBuf;

use edgerun_smtp::relay::queue::{MailIndex, MailStatus, RecipientStatusType};

// ===========================================================================
// Main
// ===========================================================================

#[derive(Debug)]
enum Command {
    List(PathBuf),
    Show(PathBuf, String),
    Retry(PathBuf, String),
    RetryAll(PathBuf),
    Delete(PathBuf, String),
    Stats(PathBuf),
}

fn parse_args() -> Result<Command, String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        return Err(format!(
            "Usage: {} <command> <data-root> [args...]\n\
             Commands:\n\
             1. list <data-root>                        — List all queued messages\n\
             2. show <data-root> <message-id>           — Show message details\n\
             3. retry <data-root> <message-id>          — Force retry a message\n\
             4. retry-all <data-root>                   — Retry all pending messages\n\
             5. delete <data-root> <message-id>         — Delete a message\n\
             6. stats <data-root>                       — Queue statistics",
            args[0]
        ));
    }

    let data_root = PathBuf::from(&args[2]);

    match args[1].as_str() {
        "list" => Ok(Command::List(data_root)),
        "show" => {
            let msg_id = args.get(3).ok_or("show requires <message-id>")?;
            Ok(Command::Show(data_root, msg_id.clone()))
        }
        "retry" => {
            let msg_id = args.get(3).ok_or("retry requires <message-id>")?;
            Ok(Command::Retry(data_root, msg_id.clone()))
        }
        "retry-all" => Ok(Command::RetryAll(data_root)),
        "delete" => {
            let msg_id = args.get(3).ok_or("delete requires <message-id>")?;
            Ok(Command::Delete(data_root, msg_id.clone()))
        }
        "stats" => Ok(Command::Stats(data_root)),
        cmd => Err(format!("Unknown command: {}", cmd)),
    }
}

fn main() -> Result<(), String> {
    let cmd = parse_args()?;
    let rt = edgerun_rt::Builder::new_multi_thread()
        .build()
        .map_err(|e| format!("failed to build runtime: {}", e))?;

    rt.block_on(async move {
        match cmd {
            Command::List(data_root) => cmd_list(&data_root).await,
            Command::Show(data_root, msg_id) => cmd_show(&data_root, &msg_id).await,
            Command::Retry(data_root, msg_id) => cmd_retry(&data_root, &msg_id).await,
            Command::RetryAll(data_root) => cmd_retry_all(&data_root).await,
            Command::Delete(data_root, msg_id) => cmd_delete(&data_root, &msg_id).await,
            Command::Stats(data_root) => cmd_stats(&data_root).await,
        }
    }).map_err(|e| format!("{}", e))?;

    Ok(())
}

// ===========================================================================
// Commands
// ===========================================================================

async fn cmd_list(data_root: &PathBuf) -> Result<(), String> {
    let index = MailIndex::open(data_root).await.map_err(|e| e.to_string())?;
    let messages = index.list_queued().await;

    if messages.is_empty() {
        println!("Queue is empty.");
        return Ok(());
    }

    println!("{:<30} {:<30} {:>6} {:>5}  {:<12}", "Message ID", "Sender", "Recipients", "Retry", "Status");
    println!("{}", "-".repeat(88));

    for msg in &messages {
        println!(
            "{:<30} {:<30} {:>6} {:>5}  {:<12}",
            msg.message_id,
            msg.envelope_sender,
            msg.recipients.len(),
            msg.retry_count,
            msg.status.as_str(),
        );
    }

    println!("\n{} message(s) in queue.", messages.len());
    Ok(())
}

async fn cmd_show(data_root: &PathBuf, message_id: &str) -> Result<(), String> {
    let index = MailIndex::open(data_root).await.map_err(|e| e.to_string())?;
    let msg = index.get_message(message_id).await
        .ok_or_else(|| format!("Message not found: {}", message_id))?;

    println!("Message ID:     {}", msg.message_id);
    println!("Sender:         {}", msg.envelope_sender);
    println!("Recipients:     {}", msg.recipients.join(", "));
    println!("Status:         {}", msg.status.as_str());
    println!("Retry Count:    {}/{}", msg.retry_count, msg.max_retries);
    println!("Next Retry:     {}", if msg.next_retry_time == 0 { "immediate".to_string() } else { msg.next_retry_time.to_string() });
    println!("Data Size:      {} bytes", msg.data.len());
    println!("Created:        {}", msg.created_at);
    println!("Updated:        {}", msg.updated_at);

    // Show per-recipient status
    println!("\nPer-recipient status:");
    for recipient in &msg.recipients {
        let delivered = index.is_recipient_delivered(message_id, recipient).await;
        let failed = index.get_failed_recipients(message_id).await;
        let is_failed = failed.iter().any(|r| &r.recipient == recipient);

        let status = if delivered {
            RecipientStatusType::Delivered
        } else if is_failed {
            RecipientStatusType::Failed
        } else {
            RecipientStatusType::Pending
        };

        println!("  {:<40} {:?}", recipient, status);
    }

    Ok(())
}

async fn cmd_retry(data_root: &PathBuf, message_id: &str) -> Result<(), String> {
    let index = MailIndex::open(data_root).await.map_err(|e| e.to_string())?;
    let msg = index.get_message(message_id).await
        .ok_or_else(|| format!("Message not found: {}", message_id))?;

    // Reset retry count and schedule for immediate delivery
    index.schedule_retry(message_id, 0, 0).await.map_err(|e| e.to_string())?;
    println!("Message {} queued for immediate retry.", message_id);

    Ok(())
}

async fn cmd_retry_all(data_root: &PathBuf) -> Result<(), String> {
    let index = MailIndex::open(data_root).await.map_err(|e| e.to_string())?;
    let messages = index.list_queued().await;

    let mut count = 0;
    for msg in &messages {
        index.schedule_retry(&msg.message_id, 0, 0).await.map_err(|e| e.to_string())?;
        count += 1;
    }

    println!("Queued {} message(s) for immediate retry.", count);
    Ok(())
}

async fn cmd_delete(data_root: &PathBuf, message_id: &str) -> Result<(), String> {
    let index = MailIndex::open(data_root).await.map_err(|e| e.to_string())?;
    let _msg = index.get_message(message_id).await
        .ok_or_else(|| format!("Message not found: {}", message_id))?;

    // Mark as bounced (permanent removal)
    index.mark_bounced(message_id).await.map_err(|e| e.to_string())?;
    println!("Message {} deleted from queue.", message_id);
    Ok(())
}

async fn cmd_stats(data_root: &PathBuf) -> Result<(), String> {
    let index = MailIndex::open(data_root).await.map_err(|e| e.to_string())?;
    let messages = index.list_queued().await;

    let total = messages.len();
    let queued = messages.iter().filter(|m| matches!(m.status, MailStatus::Queued)).count();
    let retrying = messages.iter().filter(|m| matches!(m.status, MailStatus::Retrying)).count();
    let sending = messages.iter().filter(|m| matches!(m.status, MailStatus::Sending)).count();

    let total_recipients: usize = messages.iter().map(|m| m.recipients.len()).sum();
    let avg_retries: f64 = if total > 0 {
        messages.iter().map(|m| m.retry_count as f64).sum::<f64>() / total as f64
    } else {
        0.0
    };

    let total_data: usize = messages.iter().map(|m| m.data.len()).sum();

    println!("Queue Statistics:");
    println!("  Total queued:       {}", total);
    println!("  Queued (new):       {}", queued);
    println!("  Retrying:           {}", retrying);
    println!("  Sending:            {}", sending);
    println!("  Total recipients:   {}", total_recipients);
    println!("  Avg retry count:    {:.1}", avg_retries);
    println!("  Total data:         {} bytes ({:.1} KB)", total_data, total_data as f64 / 1024.0);

    Ok(())
}
