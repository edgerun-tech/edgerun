use edgerun_hardware_signing::NodeID;
use edgerun_linux_netif::discover_network_interfaces;
use edgerun_network_interface::NetworkLinkState;
use edgerun_mesh::{FrameType, LocalNode};
use edgerun_mesh_link::MeshLink;
use edgerun_mesh::MeshRouter;
use edgerun_rt::mpsc::{Sender, Receiver};
use prost::Message;

use crate::types::{MeshCommandRequest, MeshReply};

pub fn run_mesh_loop(
    node_id: NodeID,
    mesh_command_tx: Sender<MeshCommandRequest>,
    mesh_reply_rx: Receiver<MeshReply>,
) {
    let local = LocalNode::new(node_id);
    let mut mesh_link = MeshLink::new();
    mesh_link.set_local_node_id(node_id);
    let mut router = MeshRouter::new(local);

    match mesh_link.enable_udp_broadcast() {
        Ok(_) => edgerun_log::info!("UDP broadcast enabled on port 47079"),
        Err(e) => edgerun_log::warn!("UDP broadcast failed: {}", e),
    }

    let interfaces = discover_network_interfaces().unwrap_or_default();
    for iface in &interfaces {
        if iface.link_state != NetworkLinkState::Up || iface.name == "lo" {
            continue;
        }
        let ifindex_path = format!("/sys/class/net/{}/ifindex", iface.name);
        let Ok(ifindex_str) = std::fs::read_to_string(&ifindex_path) else { continue; };
        let Ok(ifindex): Result<i32, _> = ifindex_str.trim().parse() else { continue; };
        if mesh_link.add_raw_ethernet(ifindex).is_ok() {
            edgerun_log::info!("opened raw socket");
        }
    }

    if let Err(e) = mesh_link.broadcast_discovery(&mut router) {
        edgerun_log::warn!("discovery broadcast failed: {}", e);
    }

    edgerun_log::info!("mesh loop running");

    let mut last_discovery = std::time::Instant::now();
    let discovery_interval = std::time::Duration::from_secs(5);
    let mut last_heartbeat = std::time::Instant::now();
    let heartbeat_interval = std::time::Duration::from_secs(10);

    loop {
        if let Err(e) = mesh_link.pump(&mut router) {
            edgerun_log::warn!("mesh pump error: {}", e);
        }

        let frames = mesh_link.drain_inbound_data_frames();
        for frame in frames {
            if frame.header.dest == node_id || frame.header.dest.0 == [0u8; 64] {
                if frame.header.frame_type == FrameType::Data {
                    // Decode as command and send to store task for processing
                    if let Ok(command) =
                        edgerun_proto::edgerun::v0::stream::CommandEnvelope::decode(&frame.payload[..])
                    {
                        let (reply_tx, reply_rx) = edgerun_rt::oneshot::channel();
                        let _ = mesh_command_tx.send(MeshCommandRequest {
                            command,
                            raw_bytes: frame.payload.clone(),
                            source: frame.header.src,
                            reply_tx,
                        });
                        // Wait for reply and queue it back through mesh
                        if let Ok(reply) = reply_rx.blocking_recv() {
                            let reply_frame = edgerun_mesh::MeshFrame::from_payload(reply.source, reply.response_bytes);
                            mesh_link.queue_frame(reply_frame);
                        }
                    }
                }
            } else if let Some(next_hop) = router.next_hop_for(&frame.header.dest) {
                let mut fwd = frame;
                fwd.header.dest = next_hop;
                mesh_link.queue_frame(fwd);
            }
        }

        // Also receive replies that were routed from other sources (TCP queries forwarded to mesh)
        while let Ok(reply) = mesh_reply_rx.try_recv() {
            let reply_frame = edgerun_mesh::MeshFrame::from_payload(reply.source, reply.response_bytes);
            mesh_link.queue_frame(reply_frame);
        }

        let now = std::time::Instant::now();

        // Time-based discovery broadcast (every 5 seconds)
        if now.duration_since(last_discovery) >= discovery_interval {
            if let Err(e) = mesh_link.broadcast_discovery(&mut router) {
                edgerun_log::warn!("discovery failed: {}", e);
            }
            last_discovery = now;
        }

        // Time-based heartbeat tick (every 10 seconds)
        if now.duration_since(last_heartbeat) >= heartbeat_interval {
            let dead = router.tick_heartbeat();
            for _d in &dead {
                edgerun_log::info!("peer dead");
            }
            last_heartbeat = now;
        }

        if let Err(e) = mesh_link.drain_pending_frames(&mut router) {
            edgerun_log::warn!("send failed: {}", e);
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
