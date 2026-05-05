#![no_std]

extern crate alloc;

use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use core::hint::black_box;

use edgerun_core::protocol::{
    capability_runtime::CapabilityRemoteEnvelope, Digest, EventType, ProtocolRecord, Signature,
};
use edgerun_email::imap::session_core::{
    ImapPeerContext, ImapSessionConfig, ImapSessionCore, RejectAllImapPolicy,
};
use edgerun_email::lmtp::session_core::{AllowAllLmtpPolicy, LmtpSessionConfig, LmtpSessionCore};
use edgerun_email::smtp::session_core::{
    AllowAllSmtpPolicy, SmtpPeerContext, SmtpSessionConfig, SmtpSessionCore,
};
use edgerun_mesh::{LocalNode, MeshFrame, MeshRoute, MeshRouter, MeshRoutingTable, NodeID};
use edgerun_remote_capability::MemoryRemoteTransport;
use edgerun_storage::core::EventLog;
use edgerun_storage::MemEventLog;
use edgerun_tftp::message::TftpMessage;

fn len_u32(len: usize) -> u32 {
    len.min(u32::MAX as usize) as u32
}

fn protocol_core_probe() -> u32 {
    let record = ProtocolRecord::Digest(Digest {
        algorithm: 1,
        value: vec![0x42; 32],
    });
    len_u32(edgerun_core::protocol::protocol_wire_bytes(&record, false).len())
}

fn baby_probe() -> u32 {
    match edgerun_sign_verify_e2e::stream_roundtrip() {
        Ok(report) => {
            report.event_count as u32
                + report.genesis_seq as u32
                + report.next_seq as u32
                + report.next_signature_len as u32
        }
        Err(_) => 0,
    }
}

fn stream_storage_probe() -> u32 {
    let stream_id = [0x11; 64];
    let mut event = edgerun_stream::genesis_event(&stream_id, 0);
    event.signature = Some(Signature {
        algorithm: 1,
        value: vec![0x22; 64],
    });
    let hash_len = edgerun_stream::compute_event_hash(&event).value.len();

    let mut log = MemEventLog::new();
    let appended = log.append_event(&event).is_ok() as u32;
    len_u32(hash_len) + appended
}

fn node_probe() -> u32 {
    match edgerun_node::NodeConfig::from_yaml(
        "stream_id: bundle\nname: Protocol Bundle\ncontrollers: []\ntrust_nodes: []\n",
    ) {
        Ok(config) => len_u32(config.stream_id.len()) + len_u32(config.controllers.len()),
        Err(_) => 0,
    }
}

fn dns_probe() -> u32 {
    let query = edgerun_dns::DnsMessage::query(
        0x1234,
        "example.com".to_string(),
        edgerun_dns::DnsRecordType::A,
    );
    let wire = query.to_wire();
    let parsed = edgerun_dns::DnsMessage::from_wire(&wire).is_ok() as u32;
    let framed = edgerun_dns::encode_dns_tcp_frame(&wire).unwrap_or_default();
    len_u32(wire.len()) + len_u32(framed.len()) + parsed
}

fn dhcp_probe() -> u32 {
    let mac = [0x02, 0x00, 0x00, 0x00, 0x00, 0x01];
    let discover = edgerun_dhcp::DhcpMessage::discover(0x1234_5678, mac);
    let wire = discover.to_wire();
    let parsed = edgerun_dhcp::DhcpMessage::from_wire(&wire).is_ok() as u32;
    len_u32(wire.len()) + parsed
}

fn dhcpv6_probe() -> u32 {
    let solicit = edgerun_dhcpv6::message::Dhcpv6Message::solicit(7, b"client-duid", 0);
    let wire = solicit.to_wire();
    let parsed = edgerun_dhcpv6::message::Dhcpv6Message::from_wire(&wire).is_ok() as u32;
    len_u32(wire.len()) + parsed
}

fn tftp_probe() -> u32 {
    let request = TftpMessage::rrq("boot/kernel");
    let wire = request.to_wire();
    let parsed = TftpMessage::from_wire(&wire).is_ok() as u32;
    len_u32(wire.len()) + parsed
}

fn email_probe() -> u32 {
    let mut smtp = SmtpSessionCore::new(SmtpSessionConfig::default(), SmtpPeerContext::default());
    let smtp_step = smtp.handle_line("EHLO example.com", &AllowAllSmtpPolicy);

    let mut imap = ImapSessionCore::new(ImapSessionConfig::default(), ImapPeerContext::default());
    let imap_step = imap.handle_line("a1 CAPABILITY", &RejectAllImapPolicy);

    let mut lmtp = LmtpSessionCore::new(LmtpSessionConfig::default());
    let lmtp_step = lmtp.handle_line("LHLO example.com", &AllowAllLmtpPolicy);

    len_u32(smtp_step.responses.len())
        + len_u32(imap_step.responses.len())
        + len_u32(lmtp_step.responses.len())
}

fn secret_service_probe() -> u32 {
    let mut store = edgerun_secret_service::MemorySecretStore::default();
    let mut core = edgerun_secret_service::SecretServiceCore::new(&mut store);
    match core.dispatch(edgerun_secret_service::SecretRequest::CreateCollection {
        collection_name: "default".to_string(),
        label: "Default".to_string(),
    }) {
        Ok(edgerun_secret_service::SecretResponse::Unit) => 1,
        _ => 0,
    }
}

fn mesh_probe() -> u32 {
    let local = NodeID([0x33; 64]);
    let remote = NodeID([0x44; 64]);
    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: remote,
        next_hop: None,
        cost: 1,
    });

    let mut router = MeshRouter::new(LocalNode::new(local));
    let discovery = router.build_discovery_frame();
    let frame = MeshFrame::from_payload(remote, vec![1, 2, 3, 4]);

    len_u32(table.len()) + len_u32(discovery.payload.len()) + len_u32(frame.to_wire().len())
}

fn mesh_session_probe() -> u32 {
    let manager = edgerun_mesh_session::SessionManager::new(NodeID([0x55; 64]));
    len_u32(manager.our_node_id().0.len())
}

fn remote_capability_probe() -> u32 {
    let (mut left, mut right) = MemoryRemoteTransport::pair();
    let envelope = CapabilityRemoteEnvelope { message: None };
    let sent = edgerun_remote_capability::RemoteCapabilityTransport::send(&mut left, envelope)
        .is_ok() as u32;
    let received = edgerun_remote_capability::RemoteCapabilityTransport::recv(&mut right)
        .ok()
        .flatten()
        .is_some() as u32;
    sent + received
}

#[no_mangle]
pub extern "C" fn edgerun_protocol_bundle_probe() -> u32 {
    black_box(protocol_core_probe())
        ^ black_box(baby_probe())
        ^ black_box(stream_storage_probe())
        ^ black_box(node_probe())
        ^ black_box(dns_probe())
        ^ black_box(dhcp_probe())
        ^ black_box(dhcpv6_probe())
        ^ black_box(tftp_probe())
        ^ black_box(email_probe())
        ^ black_box(secret_service_probe())
        ^ black_box(mesh_probe())
        ^ black_box(mesh_session_probe())
        ^ black_box(remote_capability_probe())
}

#[no_mangle]
pub extern "C" fn edgerun_protocol_bundle_family_count() -> u32 {
    13
}

#[no_mangle]
pub extern "C" fn edgerun_protocol_bundle_stream_event_type() -> i32 {
    EventType::NodeGenesis as i32
}
