// SPDX-License-Identifier: Apache-2.0
use std::time::Duration;

use anyhow::{Context, Result};
use libp2p::futures::StreamExt;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{
    gossipsub, identify, identity, noise, ping, tcp, yamux, Multiaddr, SwarmBuilder,
};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;

const ENV_ENABLED: &str = "EDGERUN_P2P_ENABLED";
const ENV_LISTEN_ADDRS: &str = "EDGERUN_P2P_LISTEN_ADDRS";
const ENV_BOOTSTRAP_PEERS: &str = "EDGERUN_P2P_BOOTSTRAP_PEERS";
const ENV_KEY_SEED_HEX: &str = "EDGERUN_P2P_KEY_SEED_HEX";
const ENV_EVENT_BUS_TOPIC: &str = "EDGERUN_P2P_EVENT_BUS_TOPIC";

#[derive(Debug, Clone)]
pub struct P2pConfig {
    pub enabled: bool,
    pub listen_addrs: Vec<Multiaddr>,
    pub bootstrap_peers: Vec<Multiaddr>,
    pub key_seed_hex: Option<String>,
    pub node_name: String,
    pub event_bus_topic: String,
}

pub struct P2pEventBusHandle {
    pub publish_tx: mpsc::UnboundedSender<Vec<u8>>,
    pub inbound_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    pub task: tokio::task::JoinHandle<()>,
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "EdgerunEvent")]
struct EdgerunBehaviour {
    ping: ping::Behaviour,
    identify: identify::Behaviour,
    gossipsub: gossipsub::Behaviour,
}

#[derive(Debug)]
enum EdgerunEvent {
    Ping(ping::Event),
    Identify(Box<identify::Event>),
    Gossipsub(Box<gossipsub::Event>),
}

impl From<ping::Event> for EdgerunEvent {
    fn from(value: ping::Event) -> Self {
        Self::Ping(value)
    }
}

impl From<identify::Event> for EdgerunEvent {
    fn from(value: identify::Event) -> Self {
        Self::Identify(Box::new(value))
    }
}

impl From<gossipsub::Event> for EdgerunEvent {
    fn from(value: gossipsub::Event) -> Self {
        Self::Gossipsub(Box::new(value))
    }
}

pub fn load_config_from_env(role: &str) -> Result<P2pConfig> {
    let enabled = read_env_bool(ENV_ENABLED, false);
    let listen_addrs = parse_multiaddr_list(
        &std::env::var(ENV_LISTEN_ADDRS).unwrap_or_else(|_| default_listen_addrs(role)),
        ENV_LISTEN_ADDRS,
    )?;
    let bootstrap_peers = parse_multiaddr_list(
        &std::env::var(ENV_BOOTSTRAP_PEERS).unwrap_or_default(),
        ENV_BOOTSTRAP_PEERS,
    )?;
    let key_seed_hex = std::env::var(ENV_KEY_SEED_HEX)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let event_bus_topic = std::env::var(ENV_EVENT_BUS_TOPIC)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "edgerun-event-bus-v1".to_string());

    Ok(P2pConfig {
        enabled,
        listen_addrs,
        bootstrap_peers,
        key_seed_hex,
        node_name: format!("edgerun-{role}"),
        event_bus_topic,
    })
}

pub async fn spawn_event_bus_from_env(role: &str) -> Result<Option<P2pEventBusHandle>> {
    let config = load_config_from_env(role)?;
    if !config.enabled {
        tracing::info!(role, "p2p disabled");
        return Ok(None);
    }

    let local_key = keypair_from_seed_hex(config.key_seed_hex.as_deref())?;
    let local_peer_id = local_key.public().to_peer_id();
    let topic = gossipsub::IdentTopic::new(config.event_bus_topic.clone());

    let gossipsub_cfg = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(5))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .build()
        .context("failed to build gossipsub config")?;

    let mut swarm = SwarmBuilder::with_existing_identity(local_key.clone())
        .with_tokio()
        .with_tcp(
            tcp::Config::default().nodelay(true),
            noise::Config::new,
            yamux::Config::default,
        )
        .context("failed to configure libp2p tcp transport")?
        .with_behaviour(|key| {
            let identify_cfg = identify::Config::new("/edgerun/p2p/1.0.0".to_string(), key.public())
                .with_agent_version(config.node_name.clone());
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(local_key),
                gossipsub_cfg,
            )
            .expect("valid gossipsub setup");
            EdgerunBehaviour {
                ping: ping::Behaviour::new(
                    ping::Config::new()
                        .with_interval(Duration::from_secs(15))
                        .with_timeout(Duration::from_secs(10)),
                ),
                identify: identify::Behaviour::new(identify_cfg),
                gossipsub,
            }
        })
        .context("failed to create libp2p behaviour")?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    swarm
        .behaviour_mut()
        .gossipsub
        .subscribe(&topic)
        .context("failed to subscribe to event bus topic")?;

    for addr in &config.listen_addrs {
        swarm
            .listen_on(addr.clone())
            .with_context(|| format!("failed to listen on p2p addr {addr}"))?;
    }

    for addr in &config.bootstrap_peers {
        if let Err(err) = swarm.dial(addr.clone()) {
            tracing::warn!(%addr, error = %err, "p2p bootstrap dial failed");
        }
    }

    tracing::info!(
        peer_id = %local_peer_id,
        listen = ?config.listen_addrs,
        bootstrap = ?config.bootstrap_peers,
        topic = %config.event_bus_topic,
        "p2p event bus runtime enabled"
    );

    let (publish_tx, mut publish_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (inbound_tx, inbound_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let publish_topic = topic.clone();

    let task = tokio::spawn(async move {
        loop {
            tokio::select! {
                maybe_payload = publish_rx.recv() => {
                    let Some(payload) = maybe_payload else {
                        break;
                    };
                    if let Err(err) = swarm.behaviour_mut().gossipsub.publish(publish_topic.clone(), payload) {
                        tracing::warn!(error = %err, "p2p event bus publish failed");
                    }
                }
                swarm_event = swarm.select_next_some() => {
                    match swarm_event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            tracing::info!(%address, "p2p listening");
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            tracing::info!(%peer_id, "p2p peer connected");
                        }
                        SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                            tracing::info!(%peer_id, ?cause, "p2p peer disconnected");
                        }
                        SwarmEvent::Behaviour(EdgerunEvent::Gossipsub(ev)) => {
                            if let gossipsub::Event::Message { message, .. } = *ev {
                                let _ = inbound_tx.send(message.data);
                            }
                        }
                        SwarmEvent::Behaviour(EdgerunEvent::Identify(ev)) => {
                            tracing::debug!(?ev, "p2p identify event");
                        }
                        SwarmEvent::Behaviour(EdgerunEvent::Ping(ev)) => {
                            tracing::debug!(?ev, "p2p ping event");
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    Ok(Some(P2pEventBusHandle {
        publish_tx,
        inbound_rx,
        task,
    }))
}

pub async fn spawn_from_env(role: &str) -> Result<Option<tokio::task::JoinHandle<()>>> {
    Ok(spawn_event_bus_from_env(role).await?.map(|runtime| runtime.task))
}

fn keypair_from_seed_hex(seed_hex: Option<&str>) -> Result<identity::Keypair> {
    match seed_hex {
        None => Ok(identity::Keypair::generate_ed25519()),
        Some(raw) => {
            let bytes = hex::decode(raw).context("invalid EDGERUN_P2P_KEY_SEED_HEX hex")?;
            if bytes.is_empty() {
                return Ok(identity::Keypair::generate_ed25519());
            }
            let digest = Sha256::digest(bytes);
            let mut seed = [0_u8; 32];
            seed.copy_from_slice(&digest[..32]);
            identity::Keypair::ed25519_from_bytes(seed)
                .map_err(|e| anyhow::anyhow!("invalid p2p seed: {e}"))
        }
    }
}

fn default_listen_addrs(role: &str) -> String {
    match role {
        "scheduler" => "/ip4/0.0.0.0/tcp/9100".to_string(),
        "worker" => "/ip4/0.0.0.0/tcp/9101".to_string(),
        _ => "/ip4/0.0.0.0/tcp/9102".to_string(),
    }
}

fn read_env_bool(key: &str, default_value: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default_value)
}

fn parse_multiaddr_list(raw: &str, var_name: &str) -> Result<Vec<Multiaddr>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    trimmed
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<Multiaddr>()
                .with_context(|| format!("invalid multiaddr in {var_name}: {s}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_multiaddr_list_ok() {
        let addrs = parse_multiaddr_list("/ip4/127.0.0.1/tcp/9000,/ip4/10.0.0.1/tcp/9100", "X")
            .expect("parse ok");
        assert_eq!(addrs.len(), 2);
    }

    #[test]
    fn parse_multiaddr_list_empty() {
        let addrs = parse_multiaddr_list("", "X").expect("parse empty ok");
        assert!(addrs.is_empty());
    }

    #[test]
    fn deterministic_keypair_from_seed() {
        let k1 = keypair_from_seed_hex(Some("deadbeef")).expect("k1");
        let k2 = keypair_from_seed_hex(Some("deadbeef")).expect("k2");
        assert_eq!(k1.public().to_peer_id(), k2.public().to_peer_id());
    }
}
