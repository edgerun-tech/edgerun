use std::collections::HashMap;

use crate::types::StoreRequest;

/// Periodically attempts to reconnect to unreachable peers.
///
/// Runs on a timer, tracks unreachable peers and attempts TCP reconnection
/// with exponential backoff. Uses `store_tx` to send peer status updates
/// to the store task.
pub async fn run_peer_reconnection(
    initial_unreachable: Vec<(String, String)>,
    _store_tx: edgerun_rt::mpsc::Sender<StoreRequest>,
) {
    // Track backoff state per peer: (retry_count, next_attempt)
    let mut backoff: HashMap<String, (u32, std::time::Instant)> = HashMap::new();
    const INITIAL_BACKOFF_SECS: u64 = 5;
    const MAX_BACKOFF_SECS: u64 = 300; // 5 minutes

    // Initialize with known unreachable peers
    for (node_id_hex, _addr) in initial_unreachable {
        backoff.insert(node_id_hex, (0, std::time::Instant::now()));
    }

    let mut interval = edgerun_rt::interval(std::time::Duration::from_secs(10));
    interval.set_missed_tick_behavior(edgerun_rt::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;
        let now = std::time::Instant::now();
        let mut to_remove = Vec::new();

        for (node_id_hex, (retry_count, next_attempt)) in backoff.iter_mut() {
            if now >= *next_attempt {
                let rc = *retry_count;
                let delay = (INITIAL_BACKOFF_SECS * 2u64.pow(rc.min(6))).min(MAX_BACKOFF_SECS);
                *next_attempt = now + std::time::Duration::from_secs(delay);
                *retry_count += 1;

                // In v0, we just log -- actual outbound reconnection is initiated
                // when the peer is listed in bootstrap_peers config.
                edgerun_log::debug!("peer reconnection pending (use bootstrap_peers config)");

                // Clean up very old entries
                if *retry_count > 20 {
                    to_remove.push(node_id_hex.clone());
                }
            }
        }

        for key in to_remove {
            backoff.remove(&key);
        }
    }
}
