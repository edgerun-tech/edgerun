//! Minimal in-memory OCI Distribution registry server for testing.
//!
//! Implements enough of the OCI Distribution Spec v2 to support
//! push/pull round-trip tests between client instances.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

/// A single stored blob (keyed by digest string like "sha256:abc...").
type BlobStore = Arc<Mutex<HashMap<String, Vec<u8>>>>;

/// Stored manifests: (repository, reference) → (media_type, body).
type ManifestStore = Arc<Mutex<HashMap<(String, String), (String, Vec<u8>)>>>;

/// Minimal OCI registry server.
///
/// Listens on a dynamically assigned port. Call [`RegistryServer::port`]
/// to get the address to connect to.
pub struct RegistryServer {
    port: u16,
    blobs: BlobStore,
    manifests: ManifestStore,
    _handle: thread::JoinHandle<()>,
}

impl RegistryServer {
    /// Start a new registry server on a random available port.
    pub fn new() -> io::Result<Self> {
        let blobs: BlobStore = Arc::new(Mutex::new(HashMap::new()));
        let manifests: ManifestStore = Arc::new(Mutex::new(HashMap::new()));

        // Bind to port 0 → OS assigns a random free port
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();

        let blobs_clone = blobs.clone();
        let manifests_clone = manifests.clone();

        let handle = thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let blobs = blobs_clone.clone();
                        let manifests = manifests_clone.clone();
                        thread::spawn(move || {
                            if let Err(e) = handle_connection(stream, &blobs, &manifests) {
                                eprintln!("[registry-server] connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("[registry-server] accept error: {}", e);
                    }
                }
            }
        });

        Ok(Self {
            port,
            blobs,
            manifests,
            _handle: handle,
        })
    }

    /// Port the server is listening on.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Address string like "127.0.0.1:PORT" — usable as the registry host.
    pub fn addr(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    /// Check if a blob exists.
    pub fn has_blob(&self, digest: &str) -> bool {
        self.blobs.lock().unwrap().contains_key(digest)
    }

    /// Check if a manifest exists.
    pub fn has_manifest(&self, repo: &str, reference: &str) -> bool {
        self.manifests.lock().unwrap().contains_key(&(repo.to_string(), reference.to_string()))
    }
}

impl Default for RegistryServer {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

fn handle_connection(
    mut stream: TcpStream,
    blobs: &BlobStore,
    manifests: &ManifestStore,
) -> io::Result<()> {
    // Read request line
    let mut buf = [0u8; 8192];
    let mut total = 0;

    loop {
        let n = stream.read(&mut buf[total..])?;
        if n == 0 {
            return Ok(());
        }
        total += n;

        // Check if we have the full headers
        if let Some(end) = buf[..total].windows(4).position(|w| w == b"\r\n\r\n") {
            let header_end = end + 4;
            return dispatch_request(&mut stream, &buf[..header_end], blobs, manifests);
        }

        if total >= buf.len() {
            return write_response(&mut stream, 413, "text/plain", b"Request too large");
        }
    }
}

fn dispatch_request(
    stream: &mut TcpStream,
    header_data: &[u8],
    blobs: &BlobStore,
    manifests: &ManifestStore,
) -> io::Result<()> {
    // Parse request line
    let header_str = match std::str::from_utf8(header_data) {
        Ok(s) => s,
        Err(_) => return write_response(stream, 400, "text/plain", b"Bad request"),
    };

    let first_line = header_str.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return write_response(stream, 400, "text/plain", b"Bad request");
    }

    let method = parts[0];
    let path = parts[1];

    // Read Content-Length if present
    let content_length: usize = header_str
        .lines()
        .find(|l| l.to_lowercase().starts_with("content-length:"))
        .and_then(|l| l.splitn(2, ':').nth(1))
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);

    // Read body if Content-Length > 0
    let body = if content_length > 0 {
        let mut body = vec![0u8; content_length];
        stream.read_exact(&mut body)?;
        body
    } else {
        Vec::new()
    };

    // Route
    match (method, path) {
        // ── Ping ───────────────────────────────────────────────
        ("GET", "/v2/") | ("HEAD", "/v2/") => {
            write_response(stream, 200, "application/json", b"{}")
        }

        // ── Get manifest ───────────────────────────────────────
        ("GET", path) if path.contains("/manifests/") => {
            get_manifest(stream, path, manifests)
        }

        // ── Push manifest ──────────────────────────────────────
        ("PUT", path) if path.contains("/manifests/") => {
            put_manifest(stream, path, &body, manifests)
        }

        // ── Get blob ───────────────────────────────────────────
        ("GET", path) if path.contains("/blobs/") => {
            get_blob(stream, path, blobs)
        }

        // ── Initiate blob upload (POST .../blobs/uploads/) ─────
        ("POST", path) if path.ends_with("/blobs/uploads/") => {
            initiate_upload(stream, path)
        }

        // ── Complete blob upload (PUT .../blobs/uploads/<uuid>?digest=...) ──
        ("PUT", path) if path.contains("/blobs/uploads/") && path.contains("digest=") => {
            complete_upload(stream, path, &body, blobs)
        }

        // ── Blob upload chunk (PATCH .../blobs/uploads/<uuid>) ─
        ("PATCH", path) if path.contains("/blobs/uploads/") => {
            // We don't track state for chunks, just accept it.
            // The PUT complete endpoint receives the full body anyway in our test client.
            write_response(stream, 202, "application/json", b"")
        }

        _ => {
            write_response(stream, 404, "text/plain", b"Not found")
        }
    }
}

fn get_manifest(
    stream: &mut TcpStream,
    path: &str,
    manifests: &ManifestStore,
) -> io::Result<()> {
    // Parse /v2/<repo>/manifests/<ref>
    let prefix = "/v2/";
    let rest = &path[prefix.len()..];
    let parts: Vec<&str> = rest.splitn(3, '/').collect();
    // parts[0] = repo, parts[1] = "manifests", parts[2] = reference
    if parts.len() < 3 || parts[1] != "manifests" {
        return write_response(stream, 400, "text/plain", b"Bad path");
    }

    let repo = parts[0];
    let reference = parts[2];
    let key = (repo.to_string(), reference.to_string());

    let store = manifests.lock().unwrap();
    match store.get(&key) {
        Some((media_type, body)) => {
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nDocker-Content-Digest: sha256:{:x}\r\nConnection: close\r\n\r\n",
                media_type,
                body.len(),
                sha256_hex(body)
            );
            stream.write_all(headers.as_bytes())?;
            stream.write_all(body)
        }
        None => {
            write_response(stream, 404, "application/json", b"{\"errors\":[{\"code\":\"MANIFEST_UNKNOWN\"}]}")
        }
    }
}

fn put_manifest(
    stream: &mut TcpStream,
    path: &str,
    body: &[u8],
    manifests: &ManifestStore,
) -> io::Result<()> {
    let prefix = "/v2/";
    let rest = &path[prefix.len()..];
    let parts: Vec<&str> = rest.splitn(3, '/').collect();
    if parts.len() < 3 || parts[1] != "manifests" {
        return write_response(stream, 400, "text/plain", b"Bad path");
    }

    let repo = parts[0];
    let reference = parts[2];

    // Detect media type from body
    let media_type = if body.contains(b"manifests") || body.contains(b"image.manifest") {
        "application/vnd.oci.image.manifest.v1+json"
    } else if body.contains(b"manifest.list") || body.contains(b"image.index") {
        "application/vnd.oci.image.index.v1+json"
    } else {
        "application/vnd.docker.distribution.manifest.v2+json"
    };

    let key = (repo.to_string(), reference.to_string());
    manifests
        .lock()
        .unwrap()
        .insert(key, (media_type.to_string(), body.to_vec()));

    let digest = sha256_hex(body);
    write_response(stream, 201, "application/json", b"{}")
}

fn get_blob(
    stream: &mut TcpStream,
    path: &str,
    blobs: &BlobStore,
) -> io::Result<()> {
    // Parse /v2/<repo>/blobs/<digest>
    let prefix = "/v2/";
    let rest = &path[prefix.len()..];
    let parts: Vec<&str> = rest.splitn(3, '/').collect();
    if parts.len() < 3 || parts[1] != "blobs" {
        return write_response(stream, 400, "text/plain", b"Bad path");
    }

    let digest = parts[2]; // e.g. "sha256:abc..."

    let store = blobs.lock().unwrap();
    match store.get(digest) {
        Some(body) => {
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nDocker-Content-Digest: {}\r\nConnection: close\r\n\r\n",
                body.len(),
                digest
            );
            stream.write_all(headers.as_bytes())?;
            stream.write_all(body)
        }
        None => {
            write_response(stream, 404, "application/json", b"{\"errors\":[{\"code\":\"BLOB_UNKNOWN\"}]}")
        }
    }
}

fn initiate_upload(stream: &mut TcpStream, path: &str) -> io::Result<()> {
    // Parse repo from /v2/<repo>/blobs/uploads/
    let prefix = "/v2/";
    let rest = &path[prefix.len()..];
    let repo = rest.strip_suffix("/blobs/uploads/").unwrap_or(rest);

    // Return a UUID-like upload location
    let uuid = "upload-session-1";
    let location = format!("/v2/{}/blobs/uploads/{}", repo, uuid);
    let headers = format!(
        "HTTP/1.1 202 Accepted\r\nLocation: {}\r\nDocker-Upload-UUID: {}\r\nRange: 0-0\r\nConnection: close\r\n\r\n",
        location, uuid
    );
    stream.write_all(headers.as_bytes())
}

fn complete_upload(
    stream: &mut TcpStream,
    path: &str,
    body: &[u8],
    blobs: &BlobStore,
) -> io::Result<()> {
    // Extract digest from query string
    let digest = if let Some(pos) = path.find("digest=") {
        let start = pos + 7;
        let end = path[start..].find('&').map(|e| start + e).unwrap_or(path.len());
        &path[start..end]
    } else {
        return write_response(stream, 400, "text/plain", b"Missing digest parameter");
    };

    blobs
        .lock()
        .unwrap()
        .insert(digest.to_string(), body.to_vec());

    let headers = format!(
        "HTTP/1.1 201 Created\r\nLocation: /v2/repo/blobs/{}\r\nDocker-Content-Digest: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        digest, digest
    );
    stream.write_all(headers.as_bytes())
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> io::Result<()> {
    let status_text = match status {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        400 => "Bad Request",
        404 => "Not Found",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        _ => "Unknown",
    };

    let headers = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status, status_text, content_type, body.len()
    );
    stream.write_all(headers.as_bytes())?;
    if !body.is_empty() {
        stream.write_all(body)?;
    }
    stream.flush()
}

fn sha256_hex(data: &[u8]) -> String {
    use edgerun_crypto::sha2::Sha256;
    use edgerun_core::bytes_to_hex;
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    let hex = bytes_to_hex(&hash);
    format!("sha256:{}", hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_starts_on_random_port() {
        let server = RegistryServer::new().unwrap();
        assert!(server.port() > 0);
    }

    #[test]
    fn addr_format() {
        let server = RegistryServer::new().unwrap();
        let addr = server.addr();
        assert!(addr.starts_with("127.0.0.1:"));
    }
}
