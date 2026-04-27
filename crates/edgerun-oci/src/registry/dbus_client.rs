//! D-Bus client for the edgerun Secret Service.
//!
//! Connects to the secret service daemon via its Unix domain socket
//! and provides a high-level API for storing and retrieving registry
//! credentials with biometric-gated access.

use crate::prelude::*;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use edgerun_secret_service::backend::Backend;
use edgerun_secret_service::dbus_types::*;
use edgerun_secret_service::dbus_wire::{decode_msg, encode_msg};

const DEFAULT_SOCKET: &str = "/run/edgerun/secret.sock";
const SERVICE_PATH: &str = "/org/freedesktop/secrets";
const SERVICE_IFACE: &str = "org.freedesktop.Secret.Service";
const COLLECTION_IFACE: &str = "org.freedesktop.Secret.Collection";
const ITEM_IFACE: &str = "org.freedesktop.Secret.Item";
const REGISTRY_COLL: &str = "/org/freedesktop/secrets/collections/registry";

/// High-level client for the edgerun Secret Service.
pub struct SecretClient {
    stream: UnixStream,
    session_path: Option<String>,
    serial: u32,
}

impl SecretClient {
    /// Connect to the secret service via its Unix domain socket.
    pub fn connect() -> io::Result<Self> {
        let socket = Path::new(DEFAULT_SOCKET);
        if !socket.exists() {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "edgerun-secret-service socket not found at /run/edgerun/secret.sock. Is the daemon running?",
            ));
        }
        let stream = UnixStream::connect(socket)?;
        Ok(Self {
            stream,
            session_path: None,
            serial: 0,
        })
    }

    /// Open a new session with the secret service.
    ///
    /// Returns the session object path. The session starts locked —
    /// call `unlock()` to trigger biometric verification.
    pub fn open_session(&mut self) -> io::Result<String> {
        self.serial += 1;
        let ser = self.serial;

        let mut msg = Msg::call(SERVICE_PATH, SERVICE_IFACE, "OpenSession", "");
        msg.ser = ser;
        msg = msg.body(
            vec![Val::S("plain".into()), Val::Var(Box::new(Val::Arr(vec![])))],
            "sv",
        );

        let reply = self.send_and_recv(&msg)?;

        if reply.mt == MType::Err {
            let err_msg = reply
                .body
                .first()
                .and_then(Val::s)
                .unwrap_or("unknown error");
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, err_msg));
        }

        if reply.body.len() >= 2 {
            if let Val::O(path) = &reply.body[1] {
                self.session_path = Some(path.clone());
                return Ok(path.clone());
            }
        }

        Err(io::Error::other("unexpected OpenSession response"))
    }

    /// Trigger biometric verification to unlock the session.
    ///
    /// This call blocks until the user completes biometric verification.
    pub fn unlock(&mut self) -> io::Result<()> {
        self.serial += 1;
        let ser = self.serial;

        let mut msg = Msg::call(SERVICE_PATH, SERVICE_IFACE, "Unlock", "");
        msg.ser = ser;
        msg = msg.body(vec![Val::Arr(vec![Val::O(SERVICE_PATH.into())])], "ao");

        let reply = self.send_and_recv(&msg)?;

        if reply.mt == MType::Err {
            let err_msg = reply
                .body
                .first()
                .and_then(Val::s)
                .unwrap_or("unlock failed");
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, err_msg));
        }

        Ok(())
    }

    /// Store a registry credential in the secret service.
    ///
    /// The credential is stored in the `registry` collection. The secret value
    /// should be in `username:password` format.
    pub fn store_registry_credential(
        &mut self,
        registry_host: &str,
        secret: &[u8],
    ) -> io::Result<()> {
        let label = format!("Registry: {}", registry_host);
        let key = Backend::item_key(&label, &[("host".to_string(), registry_host.to_string())]);
        let item_path = Backend::item_path(REGISTRY_COLL, &key);

        self.serial += 1;
        let ser = self.serial;

        // Build properties dict
        let props = vec![
            (
                Val::S("org.freedesktop.Secret.Item.Label".into()),
                Val::S(label.clone()),
            ),
            (
                Val::S("org.freedesktop.Secret.Item.Attribute.host".into()),
                Val::S(registry_host.to_string()),
            ),
        ];

        // Build secret struct: (oa{sv}ays)
        let session = self.session_path.clone().unwrap_or_default();
        let secret_bytes: Vec<Val> = secret.iter().map(|&b| Val::Y(b)).collect();
        let secret_val = Val::Str(vec![
            Val::O(session),
            Val::Dict(vec![]),
            Val::Arr(secret_bytes),
            Val::S("text/plain; charset=utf8".into()),
        ]);

        let mut msg = Msg::call(REGISTRY_COLL, COLLECTION_IFACE, "CreateItem", "");
        msg.ser = ser;
        msg = msg.body(
            vec![
                Val::Dict(props),
                secret_val,
                Val::B(true), // replace = true
            ],
            "a{sv}(oa{sv}ays)b",
        );

        let reply = self.send_and_recv(&msg)?;

        if reply.mt == MType::Err {
            let err_msg = reply
                .body
                .first()
                .and_then(Val::s)
                .unwrap_or("store failed");
            return Err(io::Error::other(err_msg));
        }

        if reply.body.is_empty() {
            return Err(io::Error::other("empty CreateItem response"));
        }

        Ok(())
    }

    /// Retrieve a registry credential from the secret service.
    ///
    /// Searches for a credential matching the given `registry_host`.
    /// If the session is locked, triggers biometric verification automatically.
    pub fn get_registry_credential(&mut self, registry_host: &str) -> io::Result<Vec<u8>> {
        let attrs = vec![(Val::S("host".into()), Val::S(registry_host.to_string()))];

        // Search for the item
        self.serial += 1;
        let ser = self.serial;

        let mut search_msg = Msg::call(SERVICE_PATH, SERVICE_IFACE, "SearchItems", "");
        search_msg.ser = ser;
        search_msg = search_msg.body(vec![Val::Dict(attrs)], "a{ss}");

        let reply = self.send_and_recv(&search_msg)?;

        if reply.mt == MType::Err {
            let err_msg = reply
                .body
                .first()
                .and_then(Val::s)
                .unwrap_or("search failed");
            return Err(io::Error::other(err_msg));
        }

        let unlocked = reply.body.first().and_then(Val::ao).unwrap_or_default();
        if unlocked.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no credential found for registry: {}", registry_host),
            ));
        }

        // Get secrets — auto-unlock if locked
        let secret = self.get_secrets_for_items(&unlocked)?;
        Ok(secret)
    }

    /// Delete a registry credential for the given registry host.
    pub fn delete_registry_credential(&mut self, registry_host: &str) -> io::Result<bool> {
        let attrs = vec![(Val::S("host".into()), Val::S(registry_host.to_string()))];

        self.serial += 1;
        let ser = self.serial;

        let mut search_msg = Msg::call(SERVICE_PATH, SERVICE_IFACE, "SearchItems", "");
        search_msg.ser = ser;
        search_msg = search_msg.body(vec![Val::Dict(attrs)], "a{ss}");

        let reply = self.send_and_recv(&search_msg)?;
        if reply.mt == MType::Err {
            return Err(io::Error::other("search failed"));
        }

        let unlocked = reply.body.first().and_then(Val::ao).unwrap_or_default();
        if unlocked.is_empty() {
            return Ok(false);
        }

        // Delete the first matching item
        let item_path = &unlocked[0];
        self.serial += 1;
        let ser = self.serial;

        let mut delete_msg = Msg::call(item_path, ITEM_IFACE, "Delete", "");
        delete_msg.ser = ser;
        delete_msg = delete_msg.body(vec![], "");

        let reply = self.send_and_recv(&delete_msg)?;
        if reply.mt == MType::Err {
            return Err(io::Error::other("delete failed"));
        }

        Ok(true)
    }

    /// Get secrets for the given item paths, auto-unlocking if needed.
    fn get_secrets_for_items(&mut self, item_paths: &[String]) -> io::Result<Vec<u8>> {
        let session = self.session_path.clone().unwrap_or_default();
        let paths: Vec<Val> = item_paths.iter().map(|p| Val::O(p.clone())).collect();

        self.serial += 1;
        let ser = self.serial;

        let mut get_msg = Msg::call(SERVICE_PATH, SERVICE_IFACE, "GetSecrets", "");
        get_msg.ser = ser;
        get_msg = get_msg.body(vec![Val::Arr(paths.clone()), Val::O(session.clone())], "ao");

        let reply = self.send_and_recv(&get_msg)?;

        if reply.mt == MType::Err {
            let err_name = reply.iface().unwrap_or("");
            if err_name.contains("IsLocked") {
                // Trigger biometric unlock
                self.unlock()?;
                // Retry
                self.serial += 1;
                let ser = self.serial;
                let mut retry_msg = Msg::call(SERVICE_PATH, SERVICE_IFACE, "GetSecrets", "");
                retry_msg.ser = ser;
                retry_msg = retry_msg.body(vec![Val::Arr(paths), Val::O(session)], "ao");

                let reply2 = self.send_and_recv(&retry_msg)?;
                if reply2.mt == MType::Err {
                    let err_msg = reply2
                        .body
                        .first()
                        .and_then(Val::s)
                        .unwrap_or("get secrets failed");
                    return Err(io::Error::new(io::ErrorKind::PermissionDenied, err_msg));
                }
                return extract_first_secret(&reply2);
            }

            let err_msg = reply
                .body
                .first()
                .and_then(Val::s)
                .unwrap_or("get secrets failed");
            return Err(io::Error::other(err_msg));
        }

        extract_first_secret(&reply)
    }

    /// Send a D-Bus message and receive the reply.
    fn send_and_recv(&mut self, msg: &Msg) -> io::Result<Msg> {
        let data = encode_msg(msg);
        self.stream.write_all(&data)?;
        self.stream.flush()?;

        let mut buf = Vec::with_capacity(4096);
        let mut chunk = [0u8; 4096];

        loop {
            let n = self.stream.read(&mut chunk)?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionReset,
                    "secret service connection closed",
                ));
            }
            buf.extend_from_slice(&chunk[..n]);

            match decode_msg(&buf) {
                Ok(reply) => return Ok(reply),
                Err(_) => continue,
            }
        }
    }
}

/// Extract the first secret from a GetSecrets reply.
fn extract_first_secret(reply: &Msg) -> io::Result<Vec<u8>> {
    if let Some(Val::Dict(entries)) = reply.body.first() {
        for (_key, val) in entries {
            if let Val::Var(inner) = val {
                if let Val::Str(fields) = inner.as_ref() {
                    if fields.len() >= 3 {
                        if let Val::Arr(bytes) = &fields[2] {
                            let secret: Vec<u8> = bytes
                                .iter()
                                .filter_map(|b| if let Val::Y(v) = b { Some(*v) } else { None })
                                .collect();
                            return Ok(secret);
                        }
                    }
                }
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no secret in response",
    ))
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;

    #[test]
    fn extract_first_secret_parses_bytes() {
        let secret_bytes: Vec<Val> = b"user:pass".iter().map(|&b| Val::Y(b)).collect();
        let secret_val = Val::Str(vec![
            Val::O("/session/s1".into()),
            Val::Dict(vec![]),
            Val::Arr(secret_bytes),
            Val::S("text/plain".into()),
        ]);

        let reply = Msg::ret(1, "test").body(
            vec![Val::Dict(vec![(
                Val::O("/org/freedesktop/secrets/collections/registry/abc".into()),
                Val::Var(Box::new(secret_val)),
            )])],
            "a{o(v)}",
        );

        let secret = extract_first_secret(&reply).unwrap();
        assert_eq!(secret, b"user:pass");
    }
}
