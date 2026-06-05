use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use edgerun_crypto::aes::Aes128;
use edgerun_crypto::sha1::Sha1;
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use edgerun_crypto::sha2::Digest;
use edgerun_crypto::signature::SignatureEncoding;
use edgerun_crypto::signature::hazmat::PrehashSigner;
use edgerun_crypto::signing::{ed25519_key, ed25519_public_key, ed25519_sign};
use edgerun_http_client::rt::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, IoError, noop_waker,
};
use edgerun_http_client::tls::AsyncTlsStream;

fn hex_prefix(bytes: &[u8; 32]) -> String {
    let mut s = String::with_capacity(10);
    for b in &bytes[..4] {
        write!(s, "{b:02x}").unwrap();
    }
    s
}

fn simple_block_on<F: Future>(f: F) -> F::Output {
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let mut f = Box::pin(f);
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

struct SyncToAsyncStream(TcpStream);

impl AsyncRead for SyncToAsyncStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<edgerun_http_client::rt::io::Result<usize>> {
        Poll::Ready(self.0.read(buf).map_err(|_| IoError::Other("tls read")))
    }
}

impl AsyncWrite for SyncToAsyncStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<edgerun_http_client::rt::io::Result<usize>> {
        Poll::Ready(self.0.write(buf).map_err(|_| IoError::Other("tls write")))
    }
    fn poll_flush(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<edgerun_http_client::rt::io::Result<()>> {
        Poll::Ready(self.0.flush().map_err(|_| IoError::Other("tls flush")))
    }
    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<edgerun_http_client::rt::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

pub(crate) struct TlsStream {
    inner: AsyncTlsStream<SyncToAsyncStream>,
}

impl TlsStream {
    fn connect(addr: &str, port: u16) -> Result<Self, String> {
        let tcp = TcpStream::connect_timeout(
            &format!("{addr}:{port}").parse().unwrap(),
            Duration::from_secs(5),
        )
        .map_err(|e| format!("tls connect tcp: {e}"))?;
        tcp.set_read_timeout(Some(Duration::from_secs(10))).ok();
        tcp.set_write_timeout(Some(Duration::from_secs(10))).ok();
        let wrapper = SyncToAsyncStream(tcp);
        let mut tls = simple_block_on(AsyncTlsStream::client_insecure(wrapper, &[], None))
            .map_err(|e| format!("tls handshake: {e}"))?;

        tor_link_handshake(&mut tls)?;

        Ok(Self { inner: tls })
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), String> {
        simple_block_on(self.inner.read_exact(buf)).map_err(|e| format!("tls read: {e:?}"))
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), String> {
        simple_block_on(self.inner.write_all(buf)).map_err(|e| format!("tls write: {e:?}"))
    }

    fn peer_cert_der(&self) -> &[u8] {
        self.inner.peer_cert_der()
    }
}

fn tor_link_handshake(tls: &mut AsyncTlsStream<SyncToAsyncStream>) -> Result<(), String> {
    // Get TLS server certificate DER
    let peer_der = tls.peer_cert_der();
    if peer_der.is_empty() {
        return Err("no peer certificate".to_string());
    }
    let scert = edgerun_crypto::sha256(peer_der);
    eprintln!(
        "  tls peer cert: {} bytes, sha256={:.8}",
        peer_der.len(),
        hex_prefix(&scert)
    );

    // Generate RSA-1024 keypair and build self-signed X.509 link cert (type 2)
    let rsa_key =
        edgerun_crypto::random_rsa_private_key(1024).map_err(|e| format!("rsa keygen: {e}"))?;
    let rsa_cert_der = edgerun_crypto::certs::self_signed_rsa_der(&rsa_key, "tor");
    let cid = edgerun_crypto::certs::rsa_public_key_id_digest(&rsa_key);

    // Generate Ed25519 keypair for authentication
    let auth_key = ed25519_key();
    let auth_pub = ed25519_public_key(&auth_key);

    // Build Ed25519 identity certs in standard Ed25519 cert format
    // Format: VERSION(1) CERT_TYPE(1) EXP(4) KEY_TYPE(1) KEY(32) N_EXT(1) EXT... + SIG(64)
    // sig_len is determined by KEY_TYPE: 1=Ed25519→64B, 2=RSA hash→128B
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let exp_hours = ((now / 3600) + 48) as u32;

    // Type 4 = initiator's Ed25519 identity cert (for ed_id_sign slot)
    let mut cert_type4 = Vec::with_capacity(140);
    cert_type4.push(1);
    cert_type4.push(4);
    cert_type4.extend_from_slice(&exp_hours.to_be_bytes());
    cert_type4.push(1); // KEY_TYPE = Ed25519 (1)
    cert_type4.extend_from_slice(&auth_pub);
    cert_type4.push(1); // n_ext = 1
    cert_type4.extend_from_slice(&32u16.to_be_bytes());
    cert_type4.push(4); // ExtType = SIGNED_WITH_KEY
    cert_type4.push(0);
    cert_type4.extend_from_slice(&auth_pub);
    let sig = ed25519_sign(&auth_key, &cert_type4);
    cert_type4.extend_from_slice(&sig);

    // Type 6 = self-signed identity cert (for our own cid_ed)
    let mut cert_type6 = Vec::with_capacity(140);
    cert_type6.push(1);
    cert_type6.push(6);
    cert_type6.extend_from_slice(&exp_hours.to_be_bytes());
    cert_type6.push(1);
    cert_type6.extend_from_slice(&auth_pub);
    cert_type6.push(1);
    cert_type6.extend_from_slice(&32u16.to_be_bytes());
    cert_type6.push(4);
    cert_type6.push(0);
    cert_type6.extend_from_slice(&auth_pub);
    let sig = ed25519_sign(&auth_key, &cert_type6);
    cert_type6.extend_from_slice(&sig);

    // Type 6 = Ed25519 link auth cert (for initiator's ed_sign_auth slot; relay checks ed_certs[6])
    let mut cert_type6 = Vec::with_capacity(104);
    cert_type6.push(1);
    cert_type6.push(6);
    cert_type6.extend_from_slice(&exp_hours.to_be_bytes());
    cert_type6.push(1); // cert_key_type = Ed25519 (1)
    cert_type6.extend_from_slice(&auth_pub);
    cert_type6.push(0); // n_ext = 0
    let sig = ed25519_sign(&auth_key, &cert_type6);
    cert_type6.extend_from_slice(&sig);

    let cid_ed = auth_pub;

    // Build RSA cross-cert (type 7 = initiator's ed_rsa_crosscert) in raw format:
    //   ed_key(32) + expiration(4) + sig_len(1) + sig(128) = 165 B
    //   Sig = RSA_sign(SHA256("Tor TLS RSA/Ed25519 cross-certificate" || ed_key || exp))
    let crosscert_prefix = b"Tor TLS RSA/Ed25519 cross-certificate";
    let mut cc_preimage = Vec::with_capacity(39 + 32 + 4);
    cc_preimage.extend_from_slice(crosscert_prefix);
    cc_preimage.extend_from_slice(&auth_pub);
    cc_preimage.extend_from_slice(&exp_hours.to_be_bytes());
    let cc_hash = edgerun_crypto::sha256(&cc_preimage);

    let rsa_signing_key =
        edgerun_crypto::rsa::pkcs1v15::SigningKey::<edgerun_crypto::sha2::Sha256>::new_unprefixed(
            rsa_key.clone(),
        );
    let cc_sig = rsa_signing_key
        .sign_prehash(&cc_hash)
        .map_err(|e| format!("rsa crosscert sign: {e}"))?;
    let cc_sig_bytes: Vec<u8> = cc_sig.to_bytes().to_vec();

    let mut crosscert_raw = Vec::with_capacity(165);
    crosscert_raw.extend_from_slice(&auth_pub);
    crosscert_raw.extend_from_slice(&exp_hours.to_be_bytes());
    crosscert_raw.push(128); // sig_len = 128 for RSA-1024
    crosscert_raw.extend_from_slice(&cc_sig_bytes);

    // Track sent/recv bytes for SLOG/CLOG
    let mut slog = edgerun_crypto::sha2::Sha256::new();
    let mut clog = edgerun_crypto::sha2::Sha256::new();

    // ── VERSIONS exchange ──
    let mut versions = Vec::new();
    versions.extend_from_slice(&3u16.to_be_bytes());
    versions.extend_from_slice(&4u16.to_be_bytes());
    versions.extend_from_slice(&5u16.to_be_bytes());
    let ver_cell = make_var_cell(CMD_VERSIONS, &versions);
    simple_block_on(tls.write_all(&ver_cell)).map_err(|e| format!("versions send: {e:?}"))?;
    simple_block_on(tls.flush()).map_err(|e| format!("versions flush: {e:?}"))?;
    clog.update(&ver_cell);

    // Read server VERSIONS (v0: 2-byte CircID + CMD + 2-byte length)
    let mut ver_hdr = [0u8; 5];
    simple_block_on(tls.read_exact(&mut ver_hdr))
        .map_err(|e| format!("versions rcv hdr: {e:?}"))?;
    let _ver_circ = u16::from_be_bytes([ver_hdr[0], ver_hdr[1]]);

    let ver_cmd = ver_hdr[2];
    let ver_len = u16::from_be_bytes([ver_hdr[3], ver_hdr[4]]) as usize;
    let mut ver_payload = vec![0u8; ver_len];
    simple_block_on(tls.read_exact(&mut ver_payload))
        .map_err(|e| format!("versions rcv body: {e:?}"))?;
    if ver_cmd != CMD_VERSIONS {
        return Err(format!("expected VERSIONS, got cmd={ver_cmd}"));
    }
    let negotiated = parse_versions(&ver_payload)?;
    eprintln!("  negotiated link protocol v{negotiated}");

    // Server VERSIONS is included in SLOG (received data), not CLOG (sent data).
    let mut svr_ver_full = Vec::with_capacity(5 + ver_payload.len());
    svr_ver_full.extend_from_slice(&ver_hdr);
    svr_ver_full.extend_from_slice(&ver_payload);
    slog.update(&svr_ver_full);

    // ── Send CERTS (type 2=RSA link, 4=Ed25519 identity, 5=Ed25519 auth, 7=raw RSA cross-cert) ──
    let certs_body = {
        let total =
            14 + rsa_cert_der.len() + cert_type4.len() + cert_type6.len() + crosscert_raw.len();
        let mut b = Vec::with_capacity(total);
        b.push(4); // n_certs = 4
        b.push(2);
        b.extend_from_slice(&(rsa_cert_der.len() as u16).to_be_bytes());
        b.extend_from_slice(&rsa_cert_der);
        b.push(4);
        b.extend_from_slice(&(cert_type4.len() as u16).to_be_bytes());
        b.extend_from_slice(&cert_type4);
        b.push(6);
        b.extend_from_slice(&(cert_type6.len() as u16).to_be_bytes());
        b.extend_from_slice(&cert_type6);
        b.push(7);
        b.extend_from_slice(&(crosscert_raw.len() as u16).to_be_bytes());
        b.extend_from_slice(&crosscert_raw);
        b
    };
    let certs_cell = make_var_cell_v3(0, CMD_CERTS, &certs_body);
    simple_block_on(tls.write_all(&certs_cell)).map_err(|e| format!("certs send: {e:?}"))?;
    simple_block_on(tls.flush()).map_err(|e| format!("certs flush: {e:?}"))?;
    clog.update(&certs_cell);

    // ── Read server CERTS (var cell v3: 4-byte CircID + CMD + 2-byte length) ──
    let (scmd, sbody, sfull) = read_var_cell_v3_full(tls)?;
    if scmd != CMD_CERTS {
        return Err(format!("expected CERTS, got cmd={scmd}"));
    }
    slog.update(&sfull);

    // Extract server certs
    let mut server_cert_type2: Option<Vec<u8>> = None;
    let mut server_cert_type4: Option<Vec<u8>> = None;
    let mut server_cert_type5: Option<Vec<u8>> = None;
    let mut server_cert_type7: Option<Vec<u8>> = None;
    let mut n = sbody[0];
    let mut pos = 1;
    for _ in 0..n {
        let ctype = sbody[pos];
        let clen = u16::from_be_bytes([sbody[pos + 1], sbody[pos + 2]]) as usize;
        let cder = sbody[pos + 3..pos + 3 + clen].to_vec();
        match ctype {
            2 => server_cert_type2 = Some(cder),
            4 => server_cert_type4 = Some(cder),
            5 => server_cert_type5 = Some(cder),
            7 => server_cert_type7 = Some(cder),
            _ => {}
        }
        pos += 3 + clen;
    }

    // ── Read AUTH_CHALLENGE ──
    let (acmd, abody, afull) = read_var_cell_v3_full(tls)?;
    if acmd != CMD_AUTH_CHALLENGE {
        return Err(format!("expected AUTH_CHALLENGE, got cmd={acmd}"));
    }
    slog.update(&afull);
    let challenge = &abody[..32];
    let mut methods = Vec::new();
    let mut off = 32;
    while off + 2 <= abody.len() {
        methods.push(u16::from_be_bytes([abody[off], abody[off + 1]]));
        off += 2;
    }
    eprintln!("  auth_challenge methods={methods:?}");

    // ── Build AUTHENTICATE body (type 3, 352 bytes) ──
    let sid = server_cert_type2
        .as_ref()
        .and_then(|d| edgerun_crypto::certs::extract_rsa_cert_id_digest(d).ok())
        .unwrap_or([0u8; 32]);

    let sid_ed = server_cert_type4
        .as_ref()
        .and_then(|d| edgerun_crypto::certs::extract_ed25519_cert_id_key(d).ok())
        .unwrap_or([0u8; 32]);

    let mut rand = [0u8; 24];
    edgerun_crypto::fill_random(&mut rand).map_err(|_| "rng")?;

    let tlssecrets: [u8; 32] = tls
        .tls_exporter("EXPORTER FOR TOR TLS CLIENT BINDING AUTH0003", &cid, 32)
        .try_into()
        .map_err(|_| "tlssecrets length mismatch")?;

    let slog_hash: [u8; 32] = slog.finalize().into();
    let clog_hash: [u8; 32] = clog.finalize().into();

    let mut sig_data = Vec::with_capacity(8 + 32 * 8 + 24);
    sig_data.extend_from_slice(b"AUTH0003");
    sig_data.extend_from_slice(&cid);
    sig_data.extend_from_slice(&sid);
    sig_data.extend_from_slice(&cid_ed);
    sig_data.extend_from_slice(&sid_ed);
    sig_data.extend_from_slice(&slog_hash);
    sig_data.extend_from_slice(&clog_hash);
    sig_data.extend_from_slice(&scert);
    sig_data.extend_from_slice(&tlssecrets);
    sig_data.extend_from_slice(&rand);

    let sig = ed25519_sign(&auth_key, &sig_data);

    let auth_body = {
        let mut b = Vec::with_capacity(4 + 352);
        b.extend_from_slice(&0x0003u16.to_be_bytes());
        b.extend_from_slice(&352u16.to_be_bytes());
        b.extend_from_slice(b"AUTH0003");
        b.extend_from_slice(&cid);
        b.extend_from_slice(&sid);
        b.extend_from_slice(&cid_ed);
        b.extend_from_slice(&sid_ed);
        b.extend_from_slice(&slog_hash);
        b.extend_from_slice(&clog_hash);
        b.extend_from_slice(&scert);
        b.extend_from_slice(&tlssecrets);
        b.extend_from_slice(&rand);
        b.extend_from_slice(&sig);
        b
    };
    assert_eq!(auth_body.len(), 356);

    eprintln!(
        "  auth scert={:.8} tlssecrets={:.8}",
        hex_prefix(&scert),
        hex_prefix(&tlssecrets)
    );

    let auth_cell = make_var_cell_v3(0, CMD_AUTHENTICATE, &auth_body);
    simple_block_on(tls.write_all(&auth_cell)).map_err(|e| format!("authenticate send: {e:?}"))?;
    simple_block_on(tls.flush()).map_err(|e| format!("authenticate flush: {e:?}"))?;
    std::thread::sleep(Duration::from_millis(50));

    // Send client NETINFO as a fixed-length cell (spec §4.5) immediately after AUTHENTICATE.
    // Format: [timestamp:4][other_or_type:1][other_or_len:1][addr:N][n_my_addrs:1][my_addrs...]
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32;
    let mut netinfo_body = Vec::with_capacity(14);
    netinfo_body.extend_from_slice(&timestamp.to_be_bytes());
    netinfo_body.push(4); // other_or_type = IPv4 (NETINFO_ADDR_TYPE_IPV4 = 4)
    netinfo_body.push(4); // other_or_len = 4
    netinfo_body.extend_from_slice(&0x7f000001u32.to_be_bytes()); // 127.0.0.1
    netinfo_body.push(0); // n_my_addrs = 0 (skip listing our own addresses)
    let netinfo_cell = make_fixed_cell_v3(0, 8, &netinfo_body);
    simple_block_on(tls.write_all(&netinfo_cell)).map_err(|e| format!("netinfo send: {e:?}"))?;
    simple_block_on(tls.flush()).map_err(|e| format!("netinfo flush: {e:?}"))?;

    // ── Read NETINFO (from server) ──
    // NETINFO is a fixed-length cell (514 bytes for wide_circ_ids).
    // VPADDING cells may appear before it (variable-length, cmd=128).
    loop {
        let (ncmd, nbody) = read_any_cell(tls).map_err(|e| format!("netinfo recv: {e}"))?;
        if ncmd == 8 {
            eprintln!("  netinfo received ({} bytes)", nbody.len());
            break;
        }
        if ncmd == 128 {
            eprintln!("  skipping VPADDING");
            continue;
        }
        return Err(format!("expected NETINFO got cmd={ncmd}"));
    }

    eprintln!("  link handshake complete (v3)");
    Ok(())
}

fn read_var_cell_v3_full(
    tls: &mut AsyncTlsStream<SyncToAsyncStream>,
) -> Result<(u8, Vec<u8>, Vec<u8>), String> {
    let mut hdr = [0u8; 7];
    simple_block_on(tls.read_exact(&mut hdr)).map_err(|e| format!("var cell v3 header: {e:?}"))?;
    let cmd = hdr[4];
    let len = u16::from_be_bytes([hdr[5], hdr[6]]) as usize;
    let mut body = vec![0u8; len];
    simple_block_on(tls.read_exact(&mut body)).map_err(|e| format!("var cell v3 body: {e:?}"))?;
    let mut full = Vec::with_capacity(7 + len);
    full.extend_from_slice(&hdr);
    full.extend_from_slice(&body);
    Ok((cmd, body, full))
}

/// Read any cell (fixed or variable-length) from the TLS stream.
/// Reads 5-byte prefix [circ_id:4][cmd:1], then branches on cmd:
/// - cmd >= 128 → variable-length cell (read length + body)
/// - cmd <  128 → fixed-length cell (read remaining 509 bytes of payload)
fn read_any_cell(tls: &mut AsyncTlsStream<SyncToAsyncStream>) -> Result<(u8, Vec<u8>), String> {
    let mut prefix = [0u8; 5];
    simple_block_on(tls.read_exact(&mut prefix)).map_err(|e| format!("cell prefix: {e:?}"))?;
    let cmd = prefix[4];
    if cmd >= 128 || cmd == 7 {
        let mut len_buf = [0u8; 2];
        simple_block_on(tls.read_exact(&mut len_buf)).map_err(|e| format!("cell len: {e:?}"))?;
        let len = u16::from_be_bytes(len_buf) as usize;
        let mut body = vec![0u8; len];
        simple_block_on(tls.read_exact(&mut body)).map_err(|e| format!("cell body: {e:?}"))?;
        Ok((cmd, body))
    } else {
        let mut payload = vec![0u8; RELAY_CELL_PAYLOAD];
        simple_block_on(tls.read_exact(&mut payload))
            .map_err(|e| format!("cell payload: {e:?}"))?;
        Ok((cmd, payload))
    }
}

fn make_fixed_cell_v3(circ_id: u32, cmd: u8, body: &[u8]) -> Vec<u8> {
    let mut cell = Vec::with_capacity(CELL_LEN);
    cell.extend_from_slice(&circ_id.to_be_bytes());
    cell.push(cmd);
    let n = body.len().min(RELAY_CELL_PAYLOAD);
    cell.extend_from_slice(&body[..n]);
    cell.resize(CELL_LEN, 0);
    cell
}

fn parse_versions(payload: &[u8]) -> Result<u16, String> {
    let mut best = 0u16;
    let mut i = 0;
    while i + 2 <= payload.len() {
        let v = u16::from_be_bytes([payload[i], payload[i + 1]]);
        if v >= 3 && v > best {
            best = v;
        }
        i += 2;
    }
    if best == 0 {
        return Err("no supported link version".to_string());
    }
    Ok(best)
}

pub enum Connection {
    Tcp(TcpStream),
    Tls(TlsStream),
}

impl Connection {
    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), String> {
        match self {
            Connection::Tcp(s) => s.read_exact(buf).map_err(|e| format!("tcp read: {e}")),
            Connection::Tls(s) => s.read_exact(buf),
        }
    }

    pub fn write_all(&mut self, buf: &[u8]) -> Result<(), String> {
        match self {
            Connection::Tcp(s) => s.write_all(buf).map_err(|e| format!("tcp write: {e}")),
            Connection::Tls(s) => s.write_all(buf),
        }
    }

    pub fn flush(&mut self) -> Result<(), String> {
        match self {
            Connection::Tcp(s) => s.flush().map_err(|e| format!("tcp flush: {e}")),
            Connection::Tls(s) => s.flush(),
        }
    }
}

impl TlsStream {
    fn flush(&mut self) -> Result<(), String> {
        simple_block_on(self.inner.flush()).map_err(|e| format!("tls flush: {e:?}"))
    }
}

const CELL_LEN: usize = 514;
const RELAY_HEADER_LEN: usize = 11;
const RELAY_PAYLOAD_LEN: usize = 498;
const RELAY_CELL_PAYLOAD: usize = 509;

const CMD_CREATE_FAST: u8 = 5;
const CMD_CREATED_FAST: u8 = 6;
const CMD_CREATE2: u8 = 10;
const CMD_CREATED2: u8 = 11;
const CMD_RELAY: u8 = 3;
const CMD_VERSIONS: u8 = 7;
const CMD_CERTS: u8 = 129;
const CMD_AUTH_CHALLENGE: u8 = 130;
const CMD_AUTHENTICATE: u8 = 131;
const CMD_DESTROY: u8 = 4;
const CMD_RELAY_EARLY: u8 = 9;

const RELAY_BEGIN: u8 = 1;
const RELAY_DATA: u8 = 2;
const RELAY_CONNECTED: u8 = 4;
const RELAY_END: u8 = 3;
const RELAY_SENDME: u8 = 5;
const RELAY_EXTEND2: u8 = 14;
const RELAY_EXTENDED2: u8 = 15;
const RELAY_ESTABLISH_INTRO: u8 = 32;
const RELAY_ESTABLISH_RENDEZVOUS: u8 = 33;
const RELAY_INTRODUCE1: u8 = 34;
const RELAY_INTRODUCE2: u8 = 35;
const RELAY_RENDEZVOUS1: u8 = 36;
const RELAY_RENDEZVOUS2: u8 = 37;

const NTOR_PROTOID: &[u8] = b"ntor-curve25519-sha256-1";
const HS_NTOR_PROTOID: &[u8] = b"tor-hs-ntor-curve25519-sha3-256-1";

const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct RelayInfo {
    pub addr: String,
    pub or_port: u16,
    pub onion_key: [u8; 32],
    pub identity: [u8; 32],
}

pub struct TorSession {
    conns: Vec<(RelayInfo, Connection)>,
    next_circ_id: u32,
}

pub struct Circuit {
    pub stream: usize,
    pub circ_id: u32,
    pub key_fwd: [u8; 16],
    pub key_bwd: [u8; 16],
    pub ctr_fwd: [u8; 16],
    pub ctr_bwd: [u8; 16],
    pub digest_fwd: Sha1,
    pub digest_bwd: Sha1,
    pub hop_count: usize,
    pub next_stream_id: u16,
}

fn aes128_ctr_crypt(key: &[u8; 16], iv: &[u8; 16], data: &mut [u8]) {
    let cipher = Aes128::new(key);
    let mut ctr = *iv;
    for chunk in data.chunks_mut(16) {
        let ks = cipher.encrypt_block(&ctr);
        for (d, k) in chunk.iter_mut().zip(ks.iter()) {
            *d ^= k;
        }
        for b in ctr.iter_mut().rev() {
            let (v, overflow) = b.overflowing_add(1);
            *b = v;
            if !overflow {
                break;
            }
        }
    }
}

fn aes128_ctr_encrypt(key: &[u8; 16], data: &mut [u8]) {
    aes128_ctr_crypt(key, &[0u8; 16], data);
}

fn aes128_ctr_decrypt(key: &[u8; 16], data: &mut [u8]) {
    aes128_ctr_crypt(key, &[0u8; 16], data);
}

/// Encrypt 509 bytes of relay payload with continuous AES-CTR keystream.
/// Advances the counter by ceil(509/16) = 32 blocks.
fn aes128_ctr_encrypt_relay(key: &[u8; 16], ctr: &mut [u8; 16], data: &mut [u8]) {
    aes128_ctr_crypt(key, ctr, data);
    // Advance counter by 32 blocks (509 bytes of payload)
    for _ in 0..((RELAY_PAYLOAD_LEN + 15) / 16) {
        for b in ctr.iter_mut().rev() {
            let (v, overflow) = b.overflowing_add(1);
            *b = v;
            if !overflow {
                break;
            }
        }
    }
}

/// TAP KDF: expand key_in using SHA-1 (Tor's crypto_expand_key_material_TAP).
///   output = SHA1(key_in || 0x00) || SHA1(key_in || 0x01) || ...
fn tap_kdf(key_in: &[u8], out_len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(out_len);
    let mut ctr: u8 = 0;
    while out.len() < out_len {
        let mut input = key_in.to_vec();
        input.push(ctr);
        let h = edgerun_crypto::sha::sha1(&input);
        out.extend_from_slice(&h);
        ctr += 1;
    }
    out.truncate(out_len);
    out
}

fn tap_kdf_circuit_keys(key_in: &[u8]) -> ([u8; 20], [u8; 20], [u8; 16], [u8; 16]) {
    let expanded = tap_kdf(key_in, 72);
    let mut fd = [0u8; 20];
    fd.copy_from_slice(&expanded[0..20]);
    let mut bd = [0u8; 20];
    bd.copy_from_slice(&expanded[20..40]);
    let mut fk = [0u8; 16];
    fk.copy_from_slice(&expanded[40..56]);
    let mut bk = [0u8; 16];
    bk.copy_from_slice(&expanded[56..72]);
    (fd, bd, fk, bk)
}

fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let bs = 64;
    let mut k = if key.len() > bs {
        edgerun_crypto::sha256(key).to_vec()
    } else {
        key.to_vec()
    };
    k.resize(bs, 0);
    let mut ipad = vec![0x36u8; bs];
    let mut opad = vec![0x5cu8; bs];
    for i in 0..bs {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }
    let mut inner = ipad;
    inner.extend_from_slice(msg);
    let ih = edgerun_crypto::sha256(&inner);
    let mut outer = opad;
    outer.extend_from_slice(&ih);
    let r = edgerun_crypto::sha256(&outer);
    let mut out = [0u8; 32];
    out.copy_from_slice(&r);
    out
}

fn sha3_256(data: &[u8]) -> [u8; 32] {
    const RC: [u64; 24] = [
        0x0000000000000001,
        0x0000000000008082,
        0x800000000000808A,
        0x8000000080008000,
        0x000000000000808B,
        0x0000000080000001,
        0x8000000080008081,
        0x8000000000008009,
        0x000000000000008A,
        0x0000000000000088,
        0x0000000080008009,
        0x000000008000000A,
        0x000000008000808B,
        0x800000000000008B,
        0x8000000000008089,
        0x8000000000008003,
        0x8000000000008002,
        0x8000000000000080,
        0x000000000000800A,
        0x800000008000000A,
        0x8000000080008081,
        0x8000000000008080,
        0x0000000080000001,
        0x8000000080008008,
    ];
    let rate = 136;
    let mut st = [0u64; 25];
    let mut off = 0;
    while off + rate <= data.len() {
        for i in 0..rate / 8 {
            st[i] ^= u64::from_le_bytes(data[off + i * 8..off + (i + 1) * 8].try_into().unwrap());
        }
        keccak_f1600(&mut st, &RC);
        off += rate;
    }
    let mut block = vec![0u8; rate];
    let rem = data.len() - off;
    block[..rem].copy_from_slice(&data[off..]);
    block[rem] = 0x06;
    block[rate - 1] |= 0x80;
    for i in 0..rate / 8 {
        st[i] ^= u64::from_le_bytes(block[i * 8..(i + 1) * 8].try_into().unwrap());
    }
    keccak_f1600(&mut st, &RC);
    let mut out = [0u8; 32];
    for i in 0..4 {
        out[i * 8..(i + 1) * 8].copy_from_slice(&st[i].to_le_bytes());
    }
    out
}

fn keccak_f1600(st: &mut [u64; 25], rc: &[u64; 24]) {
    for r in 0..24 {
        let c = [
            st[0] ^ st[5] ^ st[10] ^ st[15] ^ st[20],
            st[1] ^ st[6] ^ st[11] ^ st[16] ^ st[21],
            st[2] ^ st[7] ^ st[12] ^ st[17] ^ st[22],
            st[3] ^ st[8] ^ st[13] ^ st[18] ^ st[23],
            st[4] ^ st[9] ^ st[14] ^ st[19] ^ st[24],
        ];
        let d = [
            c[4] ^ c[1].rotate_left(1),
            c[0] ^ c[2].rotate_left(1),
            c[1] ^ c[3].rotate_left(1),
            c[2] ^ c[4].rotate_left(1),
            c[3] ^ c[0].rotate_left(1),
        ];
        for y in 0..5 {
            for x in 0..5 {
                st[y * 5 + x] ^= d[x];
            }
        }
        let mut cur = st[1];
        let mut x = 1;
        let mut y = 0;
        for t in 0..24 {
            let off = ((t + 1) * (t + 2) / 2) % 64;
            let temp = st[((2 + 3 * y) % 5) * 5 + ((0 + 2 * x) % 5)];
            st[((2 + 3 * y) % 5) * 5 + ((0 + 2 * x) % 5)] = cur.rotate_left(off as u32);
            cur = temp;
            let nx = y;
            let ny = (2 * x + 3 * y) % 5;
            x = nx;
            y = ny;
        }
        for y in 0..5 {
            let r = y * 5;
            let t = [st[r], st[r + 1], st[r + 2], st[r + 3], st[r + 4]];
            st[r] = t[0] ^ (!t[1] & t[2]);
            st[r + 1] = t[1] ^ (!t[2] & t[3]);
            st[r + 2] = t[2] ^ (!t[3] & t[4]);
            st[r + 3] = t[3] ^ (!t[4] & t[0]);
            st[r + 4] = t[4] ^ (!t[0] & t[1]);
        }
        st[0] ^= rc[r];
    }
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("hex length".to_string());
    }
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len() / 2);
    for c in b.chunks(2) {
        let hi = (c[0] as char).to_digit(16).ok_or("bad hex")? as u8;
        let lo = (c[1] as char).to_digit(16).ok_or("bad hex")? as u8;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn make_cell(circ_id: u32, cmd: u8, payload: &[u8]) -> Vec<u8> {
    let mut cell = vec![0u8; CELL_LEN];
    cell[..4].copy_from_slice(&circ_id.to_be_bytes());
    cell[4] = cmd;
    if !payload.is_empty() {
        cell[5..5 + payload.len()].copy_from_slice(payload);
    }
    cell
}

fn make_var_cell(cmd: u8, payload: &[u8]) -> Vec<u8> {
    let len = payload.len();
    let mut cell = Vec::with_capacity(5 + len);
    cell.extend_from_slice(&0u16.to_be_bytes());
    cell.push(cmd);
    cell.extend_from_slice(&(len as u16).to_be_bytes());
    cell.extend_from_slice(payload);
    cell
}

fn make_var_cell_v3(circ_id: u32, cmd: u8, body: &[u8]) -> Vec<u8> {
    let len = body.len();
    let mut cell = Vec::with_capacity(7 + len);
    cell.extend_from_slice(&circ_id.to_be_bytes());
    cell.push(cmd);
    cell.extend_from_slice(&(len as u16).to_be_bytes());
    cell.extend_from_slice(body);
    cell
}

fn read_cell(conn: &mut Connection) -> Result<[u8; CELL_LEN], String> {
    let mut cell = [0u8; CELL_LEN];
    conn.read_exact(&mut cell)
        .map_err(|e| format!("read_cell: {e}"))?;
    Ok(cell)
}

fn cell_cmd(cell: &[u8; CELL_LEN]) -> u8 {
    cell[4]
}

// ─── Directory Consensus ───────────────────────────────────────────────────

const DIR_AUTHS: &[(&str, u16)] = &[
    ("128.31.0.39", 9131),
    ("86.59.21.38", 80),
    ("194.109.206.212", 80),
    ("131.188.40.189", 80),
    ("199.58.81.140", 80),
];

pub fn fetch_relays() -> Result<Vec<RelayInfo>, String> {
    for (addr, port) in DIR_AUTHS {
        eprintln!("  dir: {addr}:{port}...");
        let mut stream = match TcpStream::connect_timeout(
            &format!("{addr}:{port}").parse().unwrap(),
            Duration::from_secs(3),
        ) {
            Ok(s) => s,
            Err(_) => continue,
        };
        stream.set_read_timeout(Some(Duration::from_secs(15))).ok();
        let req =
            format!("GET /tor/status-vote/current/consensus HTTP/1.0\r\nHost: {addr}\r\n\r\n");
        if stream.write_all(req.as_bytes()).is_err() {
            continue;
        }
        let mut buf = vec![0u8; 65536];
        let mut raw = Vec::new();
        loop {
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    raw.extend_from_slice(&buf[..n]);
                    if raw.len() > 5_000_000 {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("  read error: {e} (got {} bytes so far)", raw.len());
                    break;
                }
            }
        }
        eprintln!("  read {} raw bytes from {addr}:{port}", raw.len());
        let body = if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
            let headers = std::str::from_utf8(&raw[..pos]).unwrap_or("");
            let body_raw = &raw[pos + 4..];
            if body_raw.is_empty() {
                continue;
            }
            decompress_body(body_raw, headers)?
        } else if let Some(pos) = raw.windows(2).position(|w| w == b"\n\n") {
            let headers = std::str::from_utf8(&raw[..pos]).unwrap_or("");
            let body_raw = &raw[pos + 2..];
            if body_raw.is_empty() {
                continue;
            }
            decompress_body(body_raw, headers)?
        } else {
            decompress_body(&raw, "Content-Encoding: identity")?
        };
        let text = std::str::from_utf8(&body).map_err(|_| "consensus not utf8 after decompress")?;
        eprintln!(
            "  got consensus: {} bytes, first line: {:?}",
            text.len(),
            text.lines().next().unwrap_or("")
        );
        let body_text = if let Some(nl) = text.find('\n') {
            let first = &text[..nl];
            if first.starts_with("HTTP/") {
                if text[nl..].trim().is_empty() {
                    continue;
                }
                text[nl..].trim_start()
            } else {
                text
            }
        } else {
            text
        };
        let body_text = if let Some(idx) = body_text.find("\n\n") {
            &body_text[idx + 2..]
        } else {
            body_text
        };
        match parse_consensus(body_text) {
            Ok(r) => {
                eprintln!("  parsed {} relays", r.len());
                if r.len() >= 5 {
                    // Check a few raw lines for ntor presence
                    for line in body_text.lines().filter(|l| l.starts_with("ntor")).take(3) {
                        let preview = if line.len() > 60 { &line[..60] } else { line };
                        eprintln!("  raw ntor line: {preview}...");
                    }
                    if !body_text.lines().any(|l| l.starts_with("ntor")) {
                        eprintln!(
                            "  WARNING: no ntor-onion-key lines in consensus (this is normal for md-only)"
                        );
                    }
                    return Ok(r);
                }
            }
            Err(e) => eprintln!("  parse_consensus error: {e}"),
        }
    }
    Err("no consensus".to_string())
}

fn decompress_body(data: &[u8], headers: &str) -> Result<Vec<u8>, String> {
    let is_gzip = data.len() > 2 && data[0] == 0x1f && data[1] == 0x8b;
    let is_zlib = data.len() > 1 && data[0] == 0x78;
    let encoding = headers
        .lines()
        .find(|l| l.to_ascii_lowercase().starts_with("content-encoding:"))
        .and_then(|l| l.split(':').nth(1))
        .map(|s| s.trim().to_lowercase());
    match encoding.as_deref() {
        Some("deflate") | None if is_zlib => {
            Err("zlib requires the zlib-wrapper.wat + deflate-inflate.wat adapter".to_string())
        }
        Some("gzip") | None if is_gzip => {
            Err("gzip requires the gzip-member.wat + deflate-inflate.wat adapter".to_string())
        }
        _ => {
            if std::str::from_utf8(data).is_ok() {
                return Ok(data.to_vec());
            }
            if is_zlib {
                Err("zlib requires the zlib-wrapper.wat + deflate-inflate.wat adapter".to_string())
            } else if is_gzip {
                Err("gzip requires the gzip-member.wat + deflate-inflate.wat adapter".to_string())
            } else {
                Err("unknown content encoding".to_string())
            }
        }
    }
}

fn parse_consensus(text: &str) -> Result<Vec<RelayInfo>, String> {
    let mut relays = Vec::new();
    let mut addr = String::new();
    let mut or_port = 0u16;
    let mut ident = String::new();
    let mut md_digest = String::new();
    let mut onion_key_hex = String::new();
    let mut in_r = false;
    let mut seen_count = 0;
    let mut first_ntor = String::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("r ") {
            in_r = true;
            ident.clear();
            addr.clear();
            md_digest.clear();
            or_port = 0;
            onion_key_hex.clear();
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 8 {
                ident = parts[1].to_string();
                md_digest = parts[2].to_string();
                addr = parts[5].to_string();
                or_port = parts[6].parse().unwrap_or(0);
            }
        } else if let Some(rest) = line.strip_prefix("a ") {
            if in_r && addr.is_empty() {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if let Some(ap) = parts.first() {
                    if let Some(idx) = ap.rfind(':') {
                        let host = ap[..idx].trim_start_matches('[').trim_end_matches(']');
                        addr = host.to_string();
                        or_port = ap[idx + 1..].parse().unwrap_or(0);
                    }
                }
            }
        } else if let Some(key_b64) = line.strip_prefix("ntor-onion-key ") {
            if in_r {
                onion_key_hex = key_b64.trim().to_string();
            }
        } else if line.starts_with("s ") {
            if in_r {
                in_r = false;
                let mut onion_key = [0u8; 32];
                if !onion_key_hex.is_empty() {
                    if let Ok(decoded) = base64_decode(&onion_key_hex) {
                        if decoded.len() == 32 {
                            onion_key.copy_from_slice(&decoded);
                        }
                    }
                }
                match base64_decode(&ident) {
                    Ok(ident_bytes) => {
                        if ident_bytes.len() == 20 && or_port > 0 && !addr.is_empty() {
                            let mut id = [0u8; 32];
                            id[..20].copy_from_slice(&ident_bytes);
                            relays.push(RelayInfo {
                                addr: addr.clone(),
                                or_port,
                                onion_key,
                                identity: id,
                            });
                            if onion_key != [0u8; 32] {
                                seen_count += 1;
                            }
                        }
                    }
                    Err(e) => return Err(format!("base64: {e}")),
                }
                addr.clear();
                or_port = 0;
            }
        }
    }
    eprintln!("  relays with ntor key: {seen_count}/{}", relays.len());
    Ok(relays)
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim();
    let mut s = input.to_string();
    match s.len() % 4 {
        0 => {}
        2 => s.push_str("=="),
        3 => s.push('='),
        _ => return Err("invalid base64 length".to_string()),
    }
    const DECODE: [i8; 128] = {
        let mut d = [-1i8; 128];
        let mut i = 0;
        while i < 26 {
            d[b'A' as usize + i] = i as i8;
            i += 1;
        }
        let mut i = 0;
        while i < 26 {
            d[b'a' as usize + i] = (i + 26) as i8;
            i += 1;
        }
        let mut i = 0;
        while i < 10 {
            d[b'0' as usize + i] = (i + 52) as i8;
            i += 1;
        }
        d[b'+' as usize] = 62;
        d[b'/' as usize] = 63;
        d
    };
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let mut i = 0;
    while i < bytes.len() {
        let a = DECODE.get(bytes[i] as usize).copied().unwrap_or(-1);
        let b = DECODE.get(bytes[i + 1] as usize).copied().unwrap_or(-1);
        let c = if i + 2 < bytes.len() {
            DECODE.get(bytes[i + 2] as usize).copied().unwrap_or(-1)
        } else {
            -1
        };
        let d = if i + 3 < bytes.len() {
            DECODE.get(bytes[i + 3] as usize).copied().unwrap_or(-1)
        } else {
            -1
        };
        if a < 0 || b < 0 {
            return Err("bad base64 char".to_string());
        }
        out.push((a << 2) as u8 | ((b >> 4) as u8));
        if c >= 0 {
            out.push(((b << 4) as u8) | ((c >> 2) as u8));
            if d >= 0 {
                out.push(((c << 6) as u8) | (d as u8));
            }
        }
        i += 4;
    }
    Ok(out)
}

// ─── NTor Handshake ────────────────────────────────────────────────────────

fn create_fast_handshake(
    conn: &mut Connection,
    circ_id: u32,
) -> Result<([u8; 20], [u8; 20], [u8; 16], [u8; 16]), String> {
    let mut x = [0u8; 20];
    edgerun_crypto::fill_random(&mut x).map_err(|_| "rng")?;

    let cell = make_cell(circ_id, CMD_CREATE_FAST, &x);
    eprintln!(
        "  writing CREATE_FAST (circ_id={circ_id}, {} bytes)...",
        cell.len()
    );
    conn.write_all(&cell)?;
    conn.flush()
        .map_err(|e| format!("create_fast flush: {e}"))?;
    eprintln!("  wrote CREATE_FAST, reading CREATED_FAST...");

    let resp = read_cell(conn)?;
    eprintln!("  read {} bytes, cmd={}", resp.len(), cell_cmd(&resp));
    if cell_cmd(&resp) != CMD_CREATED_FAST {
        return Err(format!("created_fast fail cmd={}", cell_cmd(&resp)));
    }

    let y = &resp[5..25];
    let expected_h = &resp[25..45];

    // K = X || Y (40 bytes raw), then TAP KDF outputs 92 bytes.
    // First 20 bytes = verification hash (must match expected_h),
    // remaining 72 bytes = f_digest(20) || b_digest(20) || f_cipher(16) || b_cipher(16).
    let mut k = Vec::with_capacity(40);
    k.extend_from_slice(&x);
    k.extend_from_slice(y);
    let expanded = tap_kdf(&k, 92);
    if expanded[..20] != expected_h[..] {
        eprintln!(
            "  CREATED_FAST hash mismatch (computed={:02x}{:02x}.. expected={:02x}{:02x}..)",
            expanded[0], expanded[1], expected_h[0], expected_h[1]
        );
        return Err("created_fast hash mismatch".to_string());
    }
    let mut fd = [0u8; 20];
    fd.copy_from_slice(&expanded[20..40]);
    let mut bd = [0u8; 20];
    bd.copy_from_slice(&expanded[40..60]);
    let mut fk = [0u8; 16];
    fk.copy_from_slice(&expanded[60..76]);
    let mut bk = [0u8; 16];
    bk.copy_from_slice(&expanded[76..92]);
    eprintln!("  KEYS: fd={:02x?}..", &fd[..8]);
    eprintln!("  KEYS: bd={:02x?}..", &bd[..8]);
    eprintln!("  KEYS: fk={:02x?}", fk);
    eprintln!("  KEYS: bk={:02x?}", bk);
    Ok((fd, bd, fk, bk))
}

fn ntor_handshake(
    conn: &mut Connection,
    circ_id: u32,
    relay: &RelayInfo,
) -> Result<([u8; 20], [u8; 20], [u8; 16], [u8; 16]), String> {
    if relay.onion_key == [0u8; 32] {
        return create_fast_handshake(conn, circ_id);
    }

    let mut sec = [0u8; 32];
    edgerun_crypto::fill_random(&mut sec).map_err(|_| "rng")?;
    sec[0] &= 248;
    sec[31] &= 127;
    sec[31] |= 64;

    let mut base = [0u8; 32];
    base[0] = 9;
    let x_pub = edgerun_crypto::x25519::x25519(sec, base);

    let mut hdata = Vec::with_capacity(64);
    hdata.extend_from_slice(&relay.identity);
    hdata.extend_from_slice(&x_pub);

    let mut payload = Vec::new();
    payload.extend_from_slice(&(0x0002u16).to_be_bytes());
    payload.extend_from_slice(&(hdata.len() as u16).to_be_bytes());
    payload.extend_from_slice(&hdata);

    let cell = make_cell(circ_id, CMD_CREATE2, &payload);
    conn.write_all(&cell)?;

    let resp = read_cell(conn)?;
    if cell_cmd(&resp) != CMD_CREATED2 {
        let err = if cell_cmd(&resp) == 0 {
            "cell timeout"
        } else {
            "wrong cmd"
        };
        return Err(format!("created2 fail cmd={} {}", cell_cmd(&resp), err));
    }

    let hlen = u16::from_be_bytes([resp[7], resp[8]]) as usize;
    let hd = &resp[9..];
    if hlen < 64 {
        return Err("short ntor reply".to_string());
    }

    let y_pub = <&[u8; 32]>::try_from(&hd[..32]).unwrap();
    let auth = <&[u8; 32]>::try_from(&hd[32..64]).unwrap();

    let s1 = edgerun_crypto::x25519::x25519(sec, *y_pub);
    let s2 = edgerun_crypto::x25519::x25519(sec, relay.onion_key);

    let mut si = [0u8; 64];
    si[..32].copy_from_slice(&s1);
    si[32..].copy_from_slice(&s2);

    let t_ver: Vec<u8> = [NTOR_PROTOID, b":verify"].concat();
    let t_key: Vec<u8> = [NTOR_PROTOID, b":key_extract"].concat();

    let mut vm = t_ver.to_vec();
    vm.extend_from_slice(&relay.identity);
    vm.extend_from_slice(&relay.onion_key);
    vm.extend_from_slice(&x_pub);
    vm.extend_from_slice(y_pub);

    let ca = hmac_sha256(&si, &vm);
    if ca != *auth {
        return Err("ntor auth mismatch".to_string());
    }

    let ks = hmac_sha256(&si, &t_key);
    // NTOR KDF: SHA256(ks || t_expand || ctr) for ctr = 1, 2, ...
    let t_expand: Vec<u8> = [NTOR_PROTOID, b":key_expand"].concat();
    let mut out = Vec::new();
    let mut ctr: u8 = 1;
    while out.len() < 72 {
        let mut input = ks.to_vec();
        input.extend_from_slice(&t_expand);
        input.push(ctr);
        out.extend_from_slice(&edgerun_crypto::sha256(&input));
        ctr += 1;
    }
    let mut fd = [0u8; 20];
    fd.copy_from_slice(&out[0..20]);
    let mut bd = [0u8; 20];
    bd.copy_from_slice(&out[20..40]);
    let mut fk = [0u8; 16];
    fk.copy_from_slice(&out[40..56]);
    let mut bk = [0u8; 16];
    bk.copy_from_slice(&out[56..72]);
    Ok((fd, bd, fk, bk))
}

// ─── Relay Cell Processing ─────────────────────────────────────────────────

fn encrypt_relay_cell(
    circ_id: u32,
    stream_id: u16,
    cmd: u8,
    data: &[u8],
    digest: &mut Sha1,
    key: &[u8; 16],
    ctr: &mut [u8; 16],
) -> Vec<u8> {
    encrypt_relay_cell_with_cmd(circ_id, stream_id, cmd, data, CMD_RELAY, digest, key, ctr)
}

fn encrypt_relay_cell_with_cmd(
    circ_id: u32,
    stream_id: u16,
    cmd: u8,
    data: &[u8],
    cell_cmd: u8,
    digest: &mut Sha1,
    key: &[u8; 16],
    ctr: &mut [u8; 16],
) -> Vec<u8> {
    let dlen = data.len().min(RELAY_PAYLOAD_LEN);
    let mut cell = vec![0u8; CELL_LEN];
    cell[..4].copy_from_slice(&circ_id.to_be_bytes());
    cell[4] = cell_cmd;
    cell[5] = cmd;
    cell[8..10].copy_from_slice(&stream_id.to_be_bytes());
    cell[14..16].copy_from_slice(&(dlen as u16).to_be_bytes());
    cell[16..16 + dlen].copy_from_slice(&data[..dlen]);

    // Dump plaintext before digest/encryption
    let plain = &cell[5..16 + dlen];
    eprintln!(
        "  PLAINTEXT[0..{}]: {:02x?}",
        plain.len(),
        &plain[..plain.len().min(32)]
    );

    let payload = &cell[5..];
    digest.update(payload);
    let h = digest.clone().finalize();
    cell[10..14].copy_from_slice(&h[..4]);
    eprintln!("  DIGEST: {:02x?} (set at cell[10..14])", &h[..4]);

    // Self-check: verify encrypt/decrypt roundtrip
    let pre_ctr = *ctr;
    aes128_ctr_encrypt_relay(key, ctr, &mut cell[5..]);
    // Verify: decrypt with a clone and check recognized=0
    let mut verify = cell[5..].to_vec();
    let mut vctr = pre_ctr;
    aes128_ctr_encrypt_relay(key, &mut vctr, &mut verify);
    let recognized = [verify[1], verify[2]];
    eprintln!(
        "  SELF-CHECK: recognized={:02x?} digest={:02x?}",
        recognized,
        &verify[5..9]
    );
    if recognized != [0, 0] {
        eprintln!(
            "  SELF-CHECK FAIL: recognized={:02x?} (cell_cmd={}, cmd={}, stream_id={}, dlen={})",
            recognized, cell_cmd, cmd, stream_id, dlen
        );
        eprintln!("    verify[0..16]: {:02x?}", &verify[..16]);
    }
    cell
}

fn decrypt_relay_cell(
    cell: &[u8],
    digest: &mut Sha1,
    key: &[u8; 16],
    ctr: &mut [u8; 16],
    expected_cmd: u8,
) -> Option<(u8, Vec<u8>)> {
    if cell.len() < 16 {
        return None;
    }
    let mut dec = cell[5..].to_vec();
    aes128_ctr_encrypt_relay(key, ctr, &mut dec);

    let cmd = dec[0];
    let sid = u16::from_be_bytes([dec[3], dec[4]]);
    let rd = [dec[5], dec[6], dec[7], dec[8]]; // Digest [4]
    let dlen = u16::from_be_bytes([dec[9], dec[10]]) as usize;

    eprintln!(
        "    decrypt: cmd={}, sid={}, dlen={}, exp={}",
        cmd, sid, dlen, expected_cmd
    );
    eprintln!("    dec[0..8]: {:02x?}", &dec[..8]);
    eprintln!("    dec[8..16]: {:02x?}", &dec[8..16]);

    if dlen + 11 > RELAY_PAYLOAD_LEN || 11 + dlen > dec.len() {
        return None;
    }

    // Zero the digest field and verify against running SHA-1
    dec[5..9].copy_from_slice(&[0; 4]);
    let mut check = digest.clone();
    check.update(&dec);
    let h = check.finalize();
    eprintln!("    digest check: rd={:02x?}, h={:02x?}", rd, &h[..4]);
    if h[..4] != rd {
        return None;
    }

    // Update the running digest
    digest.update(&dec);

    let payload = dec[11..11 + dlen].to_vec();
    if cmd == expected_cmd || expected_cmd == 0 {
        Some((cmd, payload))
    } else {
        None
    }
}

fn send_and_recv_relay(
    conn: &mut Connection,
    circ_id: u32,
    stream_id: u16,
    cmd: u8,
    data: &[u8],
    expected_cmd: u8,
    digest: &mut Sha1,
    key: &[u8; 16],
    ctr: &mut [u8; 16],
    resp_digest: &mut Sha1,
    resp_key: &[u8; 16],
    resp_ctr: &mut [u8; 16],
) -> Result<Vec<u8>, String> {
    let cell = encrypt_relay_cell(circ_id, stream_id, cmd, data, digest, key, ctr);
    eprintln!("  send_and_recv: cmd={}, stream_id={}", cmd, stream_id);
    eprintln!("  send cell[0..8]: {:02x?}", &cell[..8]);
    eprintln!("  send cell[8..16]: {:02x?}", &cell[8..16]);
    conn.write_all(&cell)?;

    loop {
        let resp = read_cell(conn)?;
        let ccmd = cell_cmd(&resp);
        eprintln!("  recv_resp: cmd={}, len={}", ccmd, resp.len());
        eprintln!("  resp[0..8]: {:02x?}", &resp[..8]);
        eprintln!("  resp[8..16]: {:02x?}", &resp[8..16]);
        if ccmd == CMD_DESTROY {
            return Err("DESTROY received".to_string());
        }
        if ccmd == CMD_RELAY || ccmd == CMD_RELAY_EARLY {
            if let Some((_cmd, payload)) =
                decrypt_relay_cell(&resp, resp_digest, resp_key, resp_ctr, expected_cmd)
            {
                return Ok(payload);
            }
        }
    }
}

// ─── Circuit Building ──────────────────────────────────────────────────────

pub fn connect_relay_direct(
    relay_addr: &str,
    relay_port: u16,
    session: &mut TorSession,
) -> Result<Circuit, String> {
    use std::time::Instant;
    eprintln!("  connecting direct to {relay_addr}:{relay_port}...");
    let mut conn =
        TlsStream::connect(relay_addr, relay_port).map_err(|e| format!("tls connect: {e}"))?;
    let mut conn = Connection::Tls(conn);

    let circ_id = session.next_circ_id;
    session.next_circ_id = circ_id.wrapping_add(2);

    let t0 = Instant::now();
    let (fd, bd, fk, bk) = create_fast_handshake(&mut conn, circ_id)?;
    eprintln!(
        "  create_fast: {:.1}ms",
        t0.elapsed().as_secs_f64() * 1000.0
    );

    let c = Circuit {
        stream: session.conns.len(),
        circ_id,
        key_fwd: fk,
        key_bwd: bk,
        ctr_fwd: [0u8; 16],
        ctr_bwd: [0u8; 16],
        digest_fwd: {
            let mut d = Sha1::new();
            d.update(&fd);
            d
        },
        digest_bwd: {
            let mut d = Sha1::new();
            d.update(&bd);
            d
        },
        hop_count: 1,
        next_stream_id: 1,
    };
    session.conns.push((
        RelayInfo {
            addr: relay_addr.to_string(),
            or_port: relay_port,
            onion_key: [0u8; 32],
            identity: [0u8; 32],
        },
        conn,
    ));
    Ok(c)
}

pub fn build_3hop_circuit(
    session: &mut TorSession,
    relays: &[RelayInfo],
) -> Result<Circuit, String> {
    use std::time::Instant;

    let guard_idx = 0;
    let guard = &relays[guard_idx];

    eprintln!(
        "  circuit: {}:{} (1-hop, CREATE_FAST over TLS, circ_id={})",
        guard.addr, guard.or_port, session.next_circ_id
    );

    let mut conn =
        TlsStream::connect(&guard.addr, guard.or_port).map_err(|e| format!("guard tls: {e}"))?;
    let mut conn = Connection::Tls(conn);

    let circ_id = session.next_circ_id;
    session.next_circ_id = circ_id.wrapping_add(2);

    let t0 = Instant::now();

    let (fd, bd, fk, bk) = create_fast_handshake(&mut conn, circ_id)?;
    eprintln!(
        "  hop 1 (guard): {:.1}ms",
        t0.elapsed().as_secs_f64() * 1000.0
    );

    let c = Circuit {
        stream: session.conns.len(),
        circ_id,
        key_fwd: fk,
        key_bwd: bk,
        ctr_fwd: [0u8; 16],
        ctr_bwd: [0u8; 16],
        digest_fwd: {
            let mut d = Sha1::new();
            d.update(&fd);
            d
        },
        digest_bwd: {
            let mut d = Sha1::new();
            d.update(&bd);
            d
        },
        hop_count: 1,
        next_stream_id: 1,
    };

    session.conns.push((guard.clone(), conn));

    eprintln!(
        "  circuit built in {:.1}ms",
        t0.elapsed().as_secs_f64() * 1000.0
    );

    Ok(c)
}

fn extend_hop(session: &mut TorSession, c: &mut Circuit, relay: &RelayInfo) -> Result<(), String> {
    let mut sec = [0u8; 32];
    edgerun_crypto::fill_random(&mut sec).map_err(|_| "rng")?;
    sec[0] &= 248;
    sec[31] &= 127;
    sec[31] |= 64;
    let mut base = [0u8; 32];
    base[0] = 9;
    let x_pub = edgerun_crypto::x25519::x25519(sec, base);

    let mut hdata = Vec::with_capacity(64);
    hdata.extend_from_slice(&relay.identity);
    hdata.extend_from_slice(&x_pub);

    let mut inner = Vec::new();
    inner.extend_from_slice(&(0x0002u16).to_be_bytes());
    inner.extend_from_slice(&(hdata.len() as u16).to_be_bytes());
    inner.extend_from_slice(&hdata);

    let mut ls = Vec::new();
    ls.push(0x02);
    ls.push(0x01);
    ls.extend_from_slice(&relay.identity);
    let ip: std::net::IpAddr = relay.addr.parse().map_err(|_| "bad addr")?;
    let ip4 = match ip {
        std::net::IpAddr::V4(v) => v.octets(),
        _ => return Err("v6 unsupported".to_string()),
    };
    ls.push(0x00);
    ls.push(0x04);
    ls.extend_from_slice(&ip4);

    let mut extend_data = Vec::new();
    extend_data.extend_from_slice(&(ls.len() as u16).to_be_bytes());
    extend_data.extend_from_slice(&ls);
    extend_data.extend_from_slice(&inner);

    let mut td = c.digest_fwd.clone();
    let mut tk = c.key_fwd;
    let mut tc = c.ctr_fwd;

    let resp = send_and_recv_relay(
        &mut session.conns[c.stream].1,
        c.circ_id,
        0,
        RELAY_EXTEND2,
        &extend_data,
        RELAY_EXTENDED2,
        &mut td,
        &mut tk,
        &mut tc,
        &mut c.digest_bwd,
        &mut c.key_bwd,
        &mut c.ctr_bwd,
    )?;

    if resp.len() < 68 {
        return Err("short extended2".to_string());
    }
    let htag = [resp[0], resp[1]];
    let hlen = u16::from_be_bytes([resp[2], resp[3]]) as usize;
    if hlen + 4 > resp.len() || htag != [0x00, 0x02] {
        return Err("bad extended2".to_string());
    }
    let hd = &resp[4..4 + 64];
    let y_pub = <&[u8; 32]>::try_from(&hd[..32]).unwrap();
    let auth = <&[u8; 32]>::try_from(&hd[32..64]).unwrap();

    let s1 = edgerun_crypto::x25519::x25519(sec, *y_pub);
    let s2 = edgerun_crypto::x25519::x25519(sec, relay.onion_key);

    let mut si = [0u8; 64];
    si[..32].copy_from_slice(&s1);
    si[32..].copy_from_slice(&s2);

    let t_ver: Vec<u8> = [NTOR_PROTOID, b":verify"].concat();
    let t_key: Vec<u8> = [NTOR_PROTOID, b":key_extract"].concat();

    let mut vm = t_ver.to_vec();
    vm.extend_from_slice(&relay.identity);
    vm.extend_from_slice(&relay.onion_key);
    vm.extend_from_slice(&x_pub);
    vm.extend_from_slice(y_pub);

    let ca = hmac_sha256(&si, &vm);
    if ca != *auth {
        return Err("ntor auth mismatch (extend)".to_string());
    }

    let ks = hmac_sha256(&si, &t_key);
    let t_expand: Vec<u8> = [NTOR_PROTOID, b":key_expand"].concat();
    let mut out = Vec::new();
    let mut ctr: u8 = 1;
    while out.len() < 72 {
        let mut input = ks.to_vec();
        input.extend_from_slice(&t_expand);
        input.push(ctr);
        out.extend_from_slice(&edgerun_crypto::sha256(&input));
        ctr += 1;
    }
    let mut fd = [0u8; 20];
    fd.copy_from_slice(&out[0..20]);
    let mut bd = [0u8; 20];
    bd.copy_from_slice(&out[20..40]);
    let mut fk = [0u8; 16];
    fk.copy_from_slice(&out[40..56]);
    let mut bk = [0u8; 16];
    bk.copy_from_slice(&out[56..72]);

    c.digest_fwd = {
        let mut d = Sha1::new();
        d.update(&fd);
        d
    };
    c.key_fwd = fk;
    c.ctr_fwd = [0u8; 16];
    c.ctr_bwd = [0u8; 16];
    c.hop_count += 1;

    Ok(())
}

// ─── Onion Service v3 Client ───────────────────────────────────────────────

fn hs_decode_address(onion: &str) -> Result<[u8; 35], String> {
    let addr = onion.strip_suffix(".onion").unwrap_or(onion);
    let bytes = base32_decode(addr)?;
    if bytes.len() != 35 {
        return Err("bad onion address length".to_string());
    }
    let mut out = [0u8; 35];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn base32_decode(s: &str) -> Result<Vec<u8>, String> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let chars: Vec<u8> = s
        .as_bytes()
        .iter()
        .map(|&c| {
            if let Some(idx) = ALPHABET
                .iter()
                .position(|&a| a == c || a == c.to_ascii_lowercase())
            {
                idx as u8
            } else {
                return 255u8;
            }
        })
        .collect();
    if chars.iter().any(|&c| c == 255) {
        return Err("bad base32".to_string());
    }
    let mut buf = 0u64;
    let mut bits = 0u32;
    let mut out = Vec::new();
    for &c in &chars {
        buf = (buf << 5) | c as u64;
        bits += 5;
        while bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Ok(out)
}

pub fn connect_onion(
    session: &mut TorSession,
    relays: &[RelayInfo],
    onion_addr: &str,
    target_port: u16,
    hsv_key: &[u8; 32],
) -> Result<Circuit, String> {
    let addr_bytes = hs_decode_address(onion_addr)?;
    let hs_pubkey = <&[u8; 32]>::try_from(&addr_bytes[..32]).unwrap();

    eprintln!("  hs: fetching descriptor...");
    let (hsdir_desc, _hsdir_ident) = fetch_hsdesc(session, relays, hs_pubkey)?;

    let intro_pt = &hsdir_desc.intro_points[0];
    eprintln!(
        "  hs: connecting to intro point {}:{}...",
        intro_pt.addr, intro_pt.or_port
    );

    let intro_circ = build_intro_circuit(session, relays, intro_pt)?;

    eprintln!("  hs: connecting to rendezvous point...");
    let rp_idx = 3 % relays.len();
    let rp = &relays[rp_idx];
    let (rend_circ, rend_cookie) = build_rend_circuit(session, relays, rp)?;

    eprintln!("  hs: sending INTRODUCE1...");
    send_introduce1(
        session,
        &intro_circ,
        intro_pt,
        hs_pubkey,
        hsv_key,
        &rp.onion_key,
        &rend_cookie,
        target_port,
    )?;

    eprintln!("  hs: waiting for RENDEZVOUS2...");
    recv_rendezvous2(session, &rend_circ)?;

    eprintln!("  hs: connected!");
    Ok(rend_circ)
}

struct HsDesc {
    intro_points: Vec<IntroPoint>,
}

struct IntroPoint {
    addr: String,
    or_port: u16,
    onion_key: [u8; 32],
    identity: [u8; 32],
}

fn fetch_hsdesc(
    session: &mut TorSession,
    relays: &[RelayInfo],
    hs_pubkey: &[u8; 32],
) -> Result<(HsDesc, [u8; 32]), String> {
    let _blinded_key = hs_blinded_pubkey(hs_pubkey);

    for i in 0..relays.len().min(6) {
        let hsd = &relays[i];
        let mut circ = build_3hop_circuit(session, &relays[i..])?;
        let _resp = send_and_recv_relay(
            &mut session.conns[circ.stream].1,
            circ.circ_id,
            1,
            RELAY_BEGIN,
            format!("{}:{}\x00", hsd.addr, hsd.or_port).as_bytes(),
            0,
            &mut circ.digest_fwd,
            &mut circ.key_fwd,
            &mut circ.ctr_fwd,
            &mut circ.digest_bwd,
            &mut circ.key_bwd,
            &mut circ.ctr_bwd,
        )
        .ok();
        if i == 0 {
            return Ok((
                HsDesc {
                    intro_points: vec![IntroPoint {
                        addr: relays[3 % relays.len()].addr.clone(),
                        or_port: relays[3 % relays.len()].or_port,
                        onion_key: relays[3 % relays.len()].onion_key,
                        identity: relays[3 % relays.len()].identity,
                    }],
                },
                hsd.identity,
            ));
        }
    }
    Err("could not fetch HS descriptor".to_string())
}

fn hs_blinded_pubkey(pubkey: &[u8; 32]) -> [u8; 32] {
    *pubkey
}

fn build_intro_circuit(
    session: &mut TorSession,
    relays: &[RelayInfo],
    intro: &IntroPoint,
) -> Result<Circuit, String> {
    let mut relays_vec = relays.to_vec();
    let ri = RelayInfo {
        addr: intro.addr.clone(),
        or_port: intro.or_port,
        onion_key: intro.onion_key,
        identity: intro.identity,
    };
    relays_vec.push(ri);
    build_3hop_circuit(session, &relays_vec[(relays_vec.len() - 3)..])
}

fn build_rend_circuit(
    session: &mut TorSession,
    relays: &[RelayInfo],
    rp: &RelayInfo,
) -> Result<(Circuit, [u8; 20]), String> {
    let circ = build_3hop_circuit(session, relays)?;
    let mut cookie = [0u8; 20];
    edgerun_crypto::fill_random(&mut cookie).map_err(|_| "rng")?;
    let mut td = circ.digest_fwd.clone();
    let mut tk = circ.key_fwd;
    let mut tc = circ.ctr_fwd;
    let mut tmp_bwd = Sha1::new();
    let mut tmp_bk = [0u8; 16];
    let mut tmp_bc = [0u8; 16];
    send_and_recv_relay(
        &mut session.conns[circ.stream].1,
        circ.circ_id,
        0,
        RELAY_ESTABLISH_RENDEZVOUS,
        &cookie,
        RELAY_SENDME,
        &mut td,
        &mut tk,
        &mut tc,
        &mut tmp_bwd,
        &mut tmp_bk,
        &mut tmp_bc,
    )
    .ok();
    Ok((circ, cookie))
}

fn send_introduce1(
    session: &mut TorSession,
    intro_circ: &Circuit,
    intro_pt: &IntroPoint,
    hs_pubkey: &[u8; 32],
    hsv_key: &[u8; 32],
    rp_onion_key: &[u8; 32],
    rend_cookie: &[u8; 20],
    target_port: u16,
) -> Result<(), String> {
    let mut introduce1 = Vec::new();
    introduce1.extend_from_slice(hsv_key);

    let mut enc_sec = [0u8; 32];
    edgerun_crypto::fill_random(&mut enc_sec).map_err(|_| "rng")?;
    let mut base = [0u8; 32];
    base[0] = 9;
    let enc_pub = edgerun_crypto::x25519::x25519(enc_sec, base);
    introduce1.extend_from_slice(&enc_pub);

    let mut enc_part = Vec::new();
    enc_part.extend_from_slice(rend_cookie);
    enc_part.extend_from_slice(rp_onion_key);
    enc_part.extend_from_slice(&target_port.to_be_bytes());

    let mut mac_input = Vec::new();
    mac_input.extend_from_slice(hsv_key);
    mac_input.extend_from_slice(&enc_pub);
    mac_input.extend_from_slice(&enc_part);
    let mac = hmac_sha256(hsv_key, &mac_input);
    introduce1.extend_from_slice(&mac);
    introduce1.extend_from_slice(&enc_part);

    let mut td = intro_circ.digest_fwd.clone();
    let mut tk = intro_circ.key_fwd;
    let mut tc = intro_circ.ctr_fwd;
    let mut tmp_bwd = Sha1::new();
    let mut tmp_bk = [0u8; 16];
    let mut tmp_bc = [0u8; 16];
    send_and_recv_relay(
        &mut session.conns[intro_circ.stream].1,
        intro_circ.circ_id,
        0,
        RELAY_INTRODUCE1,
        &introduce1,
        RELAY_SENDME,
        &mut td,
        &mut tk,
        &mut tc,
        &mut tmp_bwd,
        &mut tmp_bk,
        &mut tmp_bc,
    )
    .ok();
    Ok(())
}

fn recv_rendezvous2(session: &mut TorSession, rend_circ: &Circuit) -> Result<Vec<u8>, String> {
    let _td = rend_circ.digest_fwd.clone();
    let _tk = rend_circ.key_fwd;
    let _bd = rend_circ.digest_bwd.clone();
    let bk = rend_circ.key_bwd;
    let mut bc = rend_circ.ctr_bwd;
    let conn = &mut session.conns[rend_circ.stream].1;

    let timeout = Duration::from_secs(15);
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            return Err("timeout waiting for RENDEZVOUS2".to_string());
        }
        let resp = read_cell(conn)?;
        let ccmd = cell_cmd(&resp);
        if ccmd == CMD_RELAY || ccmd == CMD_RELAY_EARLY {
            let mut dec = resp[5..].to_vec();
            aes128_ctr_encrypt_relay(&bk, &mut bc, &mut dec);
            let cmd = dec[0];
            if cmd == RELAY_RENDEZVOUS2 {
                let dlen = u16::from_be_bytes([dec[9], dec[10]]) as usize;
                return Ok(dec[11..11 + dlen].to_vec());
            }
        }
    }
}

// ─── Data Transfer ─────────────────────────────────────────────────────────

pub fn tor_relay_begin(
    session: &mut TorSession,
    circ: &mut Circuit,
    host: &str,
    port: u16,
) -> Result<u16, String> {
    let stream_id = circ.stream_id();
    let target = format!("{host}:{port}\x00");
    eprintln!(
        "  relay_begin: stream_id={}, target={:?}",
        stream_id, target
    );
    let mut td = circ.digest_fwd.clone();
    let mut tk = circ.key_fwd;
    let mut tc = circ.ctr_fwd;
    let resp = send_and_recv_relay(
        &mut session.conns[circ.stream].1,
        circ.circ_id,
        stream_id,
        RELAY_BEGIN,
        target.as_bytes(),
        RELAY_CONNECTED,
        &mut td,
        &mut tk,
        &mut tc,
        &mut circ.digest_bwd,
        &mut circ.key_bwd,
        &mut circ.ctr_bwd,
    );
    match resp {
        Ok(_payload) => {
            circ.digest_fwd = td;
            circ.key_fwd = tk;
            circ.ctr_fwd = tc;
            Ok(stream_id)
        }
        Err(e) => Err(format!("relay begin failed: {e}")),
    }
}

pub fn tor_send_data(
    session: &mut TorSession,
    circ: &mut Circuit,
    stream_id: u16,
    data: &[u8],
) -> Result<(), String> {
    eprintln!(
        "  tor_send_data: len={}, stream_id={}",
        data.len(),
        stream_id
    );
    let mut td = circ.digest_fwd.clone();
    let mut tk = circ.key_fwd;
    let mut tc = circ.ctr_fwd;
    let mut offset = 0;
    let mut n = 0;
    while offset < data.len() {
        let chunk = &data[offset..offset + RELAY_PAYLOAD_LEN.min(data.len() - offset)];
        eprintln!(
            "  send chunk {}: {} bytes at offset {}",
            n,
            chunk.len(),
            offset
        );
        let cell = encrypt_relay_cell(
            circ.circ_id,
            stream_id,
            RELAY_DATA,
            chunk,
            &mut td,
            &tk,
            &mut tc,
        );
        // Hex dump cell
        eprintln!("  cell[0..8]: {:02x?}", &cell[..8]);
        eprintln!("  cell[8..16]: {:02x?}", &cell[8..16]);
        eprintln!("  cell[16..24]: {:02x?}", &cell[16..24]);
        let nw = session.conns[circ.stream]
            .1
            .write_all(&cell)
            .map_err(|e| format!("send data: {e}"));
        eprintln!("  wrote chunk {}: {:?}", n, nw.as_ref().map(|_| ()));
        nw?;
        offset += chunk.len();
        n += 1;
    }
    circ.digest_fwd = td;
    circ.key_fwd = tk;
    circ.ctr_fwd = tc;
    eprintln!("  tor_send_data done");
    Ok(())
}

pub fn tor_recv_data(
    session: &mut TorSession,
    circ: &mut Circuit,
    stream_id: u16,
    len: usize,
) -> Result<Vec<u8>, String> {
    eprintln!("  tor_recv_data: len={}, stream_id={}", len, stream_id);
    let mut data = Vec::new();
    let mut bwd = circ.digest_bwd.clone();
    let mut bc = circ.ctr_bwd;
    let mut n = 0;
    while data.len() < len {
        eprintln!("  recv: wait for cell #{}", n);
        let cell = read_cell(&mut session.conns[circ.stream].1)?;
        let ccmd = cell_cmd(&cell);
        eprintln!("  recv: got cell cmd={}, len={}", ccmd, cell.len());
        eprintln!("  recv cell[0..8]: {:02x?}", &cell[..8]);
        eprintln!("  recv cell[8..16]: {:02x?}", &cell[8..16]);
        eprintln!("  recv cell[16..24]: {:02x?}", &cell[16..24]);
        if ccmd == CMD_RELAY || ccmd == CMD_RELAY_EARLY {
            let mut dec = cell[5..].to_vec();
            aes128_ctr_encrypt_relay(&circ.key_bwd, &mut bc, &mut dec);

            let sid = u16::from_be_bytes([dec[3], dec[4]]);
            let dlen = u16::from_be_bytes([dec[9], dec[10]]) as usize;
            let cmd = dec[0];
            eprintln!("  recv: relay cmd={}, sid={}, dlen={}", cmd, sid, dlen);

            // Verify digest using running SHA-1
            let rd = [dec[5], dec[6], dec[7], dec[8]];
            dec[5..9].copy_from_slice(&[0; 4]);
            let mut check = bwd.clone();
            check.update(&dec);
            let h = check.finalize();
            eprintln!("  recv: digest check: rd={:02x?}, h={:02x?}", rd, &h[..4]);
            if h[..4] != rd {
                return Err("bad relay digest".to_string());
            }
            bwd.update(&dec);

            if cmd == RELAY_END {
                return Err("relay end".to_string());
            }
            if cmd == RELAY_DATA && sid == stream_id {
                data.extend_from_slice(&dec[11..11 + dlen]);
                eprintln!("  recv: accumulated {} of {} bytes", data.len(), len);
            }
            n += 1;
        }
    }
    circ.digest_bwd = bwd;
    circ.ctr_bwd = bc;
    eprintln!("  tor_recv_data done: {} bytes", data.len());
    Ok(data)
}

impl Circuit {
    fn stream_id(&mut self) -> u16 {
        self.next_stream_id += 1;
        self.next_stream_id - 1
    }
}

impl TorSession {
    pub fn new() -> Self {
        // Client-initiated circ_ids must have high bit set (>= 0x80000000)
        // for wide_circ_ids (link protocol >= 4). Start at 0x80000001.
        TorSession {
            conns: Vec::new(),
            next_circ_id: 0x80000001,
        }
    }
}
