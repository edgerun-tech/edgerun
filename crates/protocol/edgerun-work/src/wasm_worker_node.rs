use alloc::vec::Vec;

use crate::channel::{ChannelEnvelope, RouteAdvertisement};
use crate::channel_order::{ChannelOrderBook, OrderedChannelEnvelope};
use crate::frame_codec::channel_envelope_from_bytes;
use crate::protocol::{Hash, NodeIdentity, NodeId, WorkPacket};
use crate::roles::{RoleContext, RoleInput, RoleOutput, WorkRole};
use crate::work_channel::{OrderedWorkChannel, WorkChannel, WorkChannelError};
use crate::ws_channel::{WsFrame, WsWorkChannel};

pub struct WasmWorkerNode<R: WorkRole> {
    pub identity: NodeIdentity,
    pub order: ChannelOrderBook,
    pub channel: WsWorkChannel,
    pub role: R,
    pub policy_hash: Hash,
}

impl<R: WorkRole> WasmWorkerNode<R> {
    pub fn new(identity: NodeIdentity, role: R, policy_hash: Hash) -> Self {
        Self {
            identity,
            order: ChannelOrderBook::new(),
            channel: WsWorkChannel::new(),
            role,
            policy_hash,
        }
    }

    pub fn add_route(&mut self, route: RouteAdvertisement) -> Result<Hash, WorkChannelError> {
        self.channel.add_route(route)
    }

    pub fn send_ordered(
        &mut self,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<OrderedChannelEnvelope, WorkChannelError> {
        self.channel
            .send_ordered(&mut self.order, self.identity.node_id, to, packet)
    }

    pub fn drain_outbound_ws_frames(&mut self) -> Vec<WsFrame> {
        self.channel.drain_outbound_frames()
    }

    pub fn drain_outbound_envelope_bytes(&mut self) -> Vec<Vec<u8>> {
        self.channel.drain_outbound_envelope_bytes()
    }

    pub fn accept_inbound_bytes(
        &mut self,
        bytes: &[u8],
        now_unix_ms: u64,
    ) -> Result<RoleOutput, WorkChannelError> {
        let envelope = channel_envelope_from_bytes(bytes).map_err(|_| WorkChannelError::PacketHashFailed)?;
        self.accept_inbound_envelope(envelope, now_unix_ms)
    }

    pub fn accept_inbound_envelope(
        &mut self,
        envelope: ChannelEnvelope,
        now_unix_ms: u64,
    ) -> Result<RoleOutput, WorkChannelError> {
        let ordered = self.channel.ordered_inbound(&mut self.order, envelope)?;
        let output = self.role.handle(
            &RoleContext {
                now_unix_ms,
                local_node: self.identity.clone(),
                policy_hash: self.policy_hash,
            },
            RoleInput {
                packet: ordered.envelope.packet,
                previous_hash: ordered.previous_message_hash,
                channel_hash: ordered.envelope.channel_id,
            },
        );
        Ok(output)
    }
}
