use edgerun_crypto::ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rkyv::{Archive, Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::io::{self, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

pub const NODE_ID_LEN: usize = 32;
pub const SIGNATURE_LEN: usize = 64;
pub const MAX_FRAME_LEN: usize = 1024 * 1024;

pub type NodeId = [u8; NODE_ID_LEN];
pub type SignatureBytes = [u8; SIGNATURE_LEN];

const REGISTER_DOMAIN: &[u8] = b"edgerun:v0:relay:register";
const SEND_DOMAIN: &[u8] = b"edgerun:v0:relay:send";

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct Register {
    pub node_id: NodeId,
    pub sequence: u64,
    pub log_head: [u8; 32],
    pub signature: SignatureBytes,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct Send {
    pub from: NodeId,
    pub to: NodeId,
    pub sequence: u64,
    pub payload: Vec<u8>,
    pub signature: SignatureBytes,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct Delivered {
    pub from: NodeId,
    pub to: NodeId,
    pub sequence: u64,
    pub received_unix_ms: u64,
    pub payload: Vec<u8>,
    pub signature: SignatureBytes,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct Ack {
    pub ok: bool,
    pub code: u16,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub enum RelayMessage {
    Register(Register),
    Send(Send),
    Delivered(Delivered),
    Ack(Ack),
}

impl RelayMessage {
    pub fn encode_rkyv(&self) -> Result<Vec<u8>, rkyv::rancor::Error> {
        rkyv::to_bytes::<rkyv::rancor::Error>(self).map(|bytes| bytes.to_vec())
    }

    pub fn decode_rkyv(bytes: &[u8]) -> Result<Self, rkyv::rancor::Error> {
        let archived = rkyv::access::<ArchivedRelayMessage, rkyv::rancor::Error>(bytes)?;
        rkyv::deserialize::<RelayMessage, rkyv::rancor::Error>(archived)
    }
}

impl Register {
    pub fn signing_preimage(&self) -> Vec<u8> {
        register_preimage(&self.node_id, self.sequence, &self.log_head)
    }

    pub fn verify(&self) -> bool {
        verify_ed25519(&self.node_id, &self.signing_preimage(), &self.signature)
    }
}

impl Send {
    pub fn signing_preimage(&self) -> Vec<u8> {
        send_preimage(&self.from, &self.to, self.sequence, &self.payload)
    }

    pub fn verify(&self) -> bool {
        verify_ed25519(&self.from, &self.signing_preimage(), &self.signature)
    }
}

pub fn register_preimage(node_id: &NodeId, sequence: u64, log_head: &[u8; 32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(REGISTER_DOMAIN.len() + 1 + NODE_ID_LEN + 8 + 32);
    out.extend_from_slice(REGISTER_DOMAIN);
    out.push(0);
    out.extend_from_slice(node_id);
    out.extend_from_slice(&sequence.to_be_bytes());
    out.extend_from_slice(log_head);
    out
}

pub fn send_preimage(from: &NodeId, to: &NodeId, sequence: u64, payload: &[u8]) -> Vec<u8> {
    let mut out =
        Vec::with_capacity(SEND_DOMAIN.len() + 1 + NODE_ID_LEN * 2 + 8 + 8 + payload.len());
    out.extend_from_slice(SEND_DOMAIN);
    out.push(0);
    out.extend_from_slice(from);
    out.extend_from_slice(to);
    out.extend_from_slice(&sequence.to_be_bytes());
    out.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

pub fn verify_ed25519(node_id: &NodeId, preimage: &[u8], signature: &SignatureBytes) -> bool {
    let Ok(public_key) = VerifyingKey::from_bytes(node_id) else {
        return false;
    };
    let sig = Signature::from_bytes(signature);
    public_key.verify(preimage, &sig).is_ok()
}

#[derive(Clone, Default)]
pub struct Relay {
    routes: Arc<Mutex<HashMap<NodeId, PeerWriter>>>,
}

#[derive(Clone)]
struct PeerWriter {
    writer: Arc<Mutex<TcpStream>>,
}

impl Relay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn serve<A: ToSocketAddrs>(&self, addr: A) -> io::Result<()> {
        let listener = TcpListener::bind(addr)?;
        self.serve_listener(listener)
    }

    pub fn serve_listener(&self, listener: TcpListener) -> io::Result<()> {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let relay = self.clone();
                    thread::spawn(move || {
                        let _ = relay.handle_stream(stream);
                    });
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub fn handle_stream(&self, mut stream: TcpStream) -> io::Result<()> {
        let writer = Arc::new(Mutex::new(stream.try_clone()?));
        let mut registered_node = None;

        loop {
            let message = match read_message(&mut stream) {
                Ok(message) => message,
                Err(error)
                    if error.kind() == ErrorKind::UnexpectedEof
                        || error.kind() == ErrorKind::ConnectionReset =>
                {
                    break;
                }
                Err(error) => {
                    let _ = write_ack(&writer, false, 400, &format!("read failed: {error}"));
                    break;
                }
            };

            match message {
                RelayMessage::Register(register) => {
                    if !register.verify() {
                        write_ack(&writer, false, 401, "invalid register signature")?;
                        continue;
                    }

                    registered_node = Some(register.node_id);
                    self.routes.lock().expect("relay routes poisoned").insert(
                        register.node_id,
                        PeerWriter {
                            writer: Arc::clone(&writer),
                        },
                    );
                    write_ack(&writer, true, 200, "registered")?;
                }
                RelayMessage::Send(send) => {
                    if !send.verify() {
                        write_ack(&writer, false, 401, "invalid send signature")?;
                        continue;
                    }

                    match self.forward(send) {
                        Ok(()) => write_ack(&writer, true, 202, "forwarded")?,
                        Err(error) => write_ack(&writer, false, error.code(), &error.to_string())?,
                    }
                }
                RelayMessage::Delivered(_) | RelayMessage::Ack(_) => {
                    write_ack(&writer, false, 400, "message type is relay-output only")?;
                }
            }
        }

        if let Some(node_id) = registered_node {
            self.remove_route_if_same_writer(node_id, &writer);
        }

        Ok(())
    }

    pub fn registered_len(&self) -> usize {
        self.routes.lock().expect("relay routes poisoned").len()
    }

    fn forward(&self, send: Send) -> Result<(), RelayError> {
        let peer = {
            let routes = self.routes.lock().expect("relay routes poisoned");
            routes.get(&send.to).cloned()
        }
        .ok_or(RelayError::NodeNotRegistered)?;

        let delivered = Delivered {
            from: send.from,
            to: send.to,
            sequence: send.sequence,
            received_unix_ms: unix_ms_now(),
            payload: send.payload,
            signature: send.signature,
        };

        write_message_locked(&peer.writer, &RelayMessage::Delivered(delivered))
            .map_err(RelayError::ForwardWrite)
    }

    fn remove_route_if_same_writer(&self, node_id: NodeId, writer: &Arc<Mutex<TcpStream>>) {
        let mut routes = self.routes.lock().expect("relay routes poisoned");
        if routes
            .get(&node_id)
            .is_some_and(|peer| Arc::ptr_eq(&peer.writer, writer))
        {
            routes.remove(&node_id);
        }
    }
}

#[derive(Debug)]
pub enum RelayError {
    NodeNotRegistered,
    ForwardWrite(io::Error),
}

impl RelayError {
    fn code(&self) -> u16 {
        match self {
            RelayError::NodeNotRegistered => 404,
            RelayError::ForwardWrite(_) => 502,
        }
    }
}

impl fmt::Display for RelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelayError::NodeNotRegistered => write!(f, "destination node is not registered"),
            RelayError::ForwardWrite(error) => write!(f, "forward write failed: {error}"),
        }
    }
}

impl std::error::Error for RelayError {}

pub fn read_message(reader: &mut impl Read) -> io::Result<RelayMessage> {
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    if len == 0 || len > MAX_FRAME_LEN {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "invalid relay frame length",
        ));
    }

    let mut bytes = vec![0u8; len];
    reader.read_exact(&mut bytes)?;
    RelayMessage::decode_rkyv(&bytes).map_err(|error| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!("invalid rkyv frame: {error}"),
        )
    })
}

pub fn write_message(writer: &mut impl Write, message: &RelayMessage) -> io::Result<()> {
    let bytes = message.encode_rkyv().map_err(|error| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!("rkyv encode failed: {error}"),
        )
    })?;
    if bytes.len() > MAX_FRAME_LEN {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "relay frame too large",
        ));
    }
    writer.write_all(&(bytes.len() as u32).to_be_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()
}

fn write_message_locked(writer: &Arc<Mutex<TcpStream>>, message: &RelayMessage) -> io::Result<()> {
    let mut writer = writer.lock().expect("relay writer poisoned");
    write_message(&mut *writer, message)
}

fn write_ack(writer: &Arc<Mutex<TcpStream>>, ok: bool, code: u16, text: &str) -> io::Result<()> {
    write_message_locked(
        writer,
        &RelayMessage::Ack(Ack {
            ok,
            code,
            text: text.to_owned(),
        }),
    )
}

fn unix_ms_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_crypto::{Ed25519SigningKey, Signer};
    use std::net::TcpListener;
    use std::time::Duration;

    fn key(byte: u8) -> Ed25519SigningKey {
        Ed25519SigningKey::from_bytes(&[byte; 32])
    }

    fn signed_register(key: &Ed25519SigningKey, sequence: u64) -> Register {
        let node_id = *key.verifying_key().as_bytes();
        let log_head = [0xA5; 32];
        let preimage = register_preimage(&node_id, sequence, &log_head);
        Register {
            node_id,
            sequence,
            log_head,
            signature: key.sign(&preimage).to_bytes(),
        }
    }

    fn signed_send(key: &Ed25519SigningKey, to: NodeId, payload: &[u8]) -> Send {
        let from = *key.verifying_key().as_bytes();
        let preimage = send_preimage(&from, &to, 7, payload);
        Send {
            from,
            to,
            sequence: 7,
            payload: payload.to_vec(),
            signature: key.sign(&preimage).to_bytes(),
        }
    }

    #[test]
    fn register_signature_verifies() {
        let key = key(9);
        let register = signed_register(&key, 1);
        assert!(register.verify());

        let mut tampered = register.clone();
        tampered.sequence += 1;
        assert!(!tampered.verify());
    }

    #[test]
    fn rkyv_frame_round_trips() {
        let key = key(1);
        let message = RelayMessage::Register(signed_register(&key, 3));
        let mut bytes = Vec::new();
        write_message(&mut bytes, &message).expect("write message");
        let recovered = read_message(&mut bytes.as_slice()).expect("read message");
        assert_eq!(message, recovered);
    }

    #[test]
    fn relay_forwards_to_registered_socket() {
        let relay = Relay::new();
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let serving_relay = relay.clone();
        thread::spawn(move || {
            let _ = serving_relay.serve_listener(listener);
        });

        let dest_key = key(2);
        let dest_id = *dest_key.verifying_key().as_bytes();
        let mut dest = TcpStream::connect(addr).expect("connect dest");
        dest.set_read_timeout(Some(Duration::from_secs(2)))
            .expect("timeout");
        write_message(
            &mut dest,
            &RelayMessage::Register(signed_register(&dest_key, 1)),
        )
        .expect("register dest");
        let ack = read_message(&mut dest).expect("read register ack");
        assert_eq!(
            ack,
            RelayMessage::Ack(Ack {
                ok: true,
                code: 200,
                text: "registered".to_owned(),
            })
        );

        let src_key = key(3);
        let mut src = TcpStream::connect(addr).expect("connect src");
        src.set_read_timeout(Some(Duration::from_secs(2)))
            .expect("timeout");
        write_message(
            &mut src,
            &RelayMessage::Send(signed_send(&src_key, dest_id, b"hello")),
        )
        .expect("send payload");

        let delivered = read_message(&mut dest).expect("read delivered");
        match delivered {
            RelayMessage::Delivered(delivered) => {
                assert_eq!(delivered.to, dest_id);
                assert_eq!(delivered.payload, b"hello");
                assert!(verify_ed25519(
                    &delivered.from,
                    &send_preimage(
                        &delivered.from,
                        &delivered.to,
                        delivered.sequence,
                        &delivered.payload
                    ),
                    &delivered.signature,
                ));
            }
            other => panic!("expected delivered frame, got {other:?}"),
        }

        let ack = read_message(&mut src).expect("read send ack");
        assert_eq!(
            ack,
            RelayMessage::Ack(Ack {
                ok: true,
                code: 202,
                text: "forwarded".to_owned(),
            })
        );
    }
}
