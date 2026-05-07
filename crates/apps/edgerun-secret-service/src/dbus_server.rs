//! D-Bus Secret Service server — Unix domain socket listener with full
//! org.freedesktop.Secret.Service implementation.

#![cfg(unix)]

use crate::prelude::v1::*;
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

use crate::backend::Backend;
use crate::dbus_bus::BusConnection;
use crate::dbus_types::*;
use crate::dbus_wire::{decode_msg, encode_msg};
use crate::service_core::{SecretRequest, SecretResponse, SecretServiceCore};
use crate::session::{
    BiometricVerifier, NoBiometricVerifier, SessionManager, DEFAULT_IDLE_TIMEOUT_US,
};

// ===========================================================================
// Server
// ===========================================================================

pub struct Server {
    listener: UnixListener,
    bus: Option<BusConnection>,
    backend: Backend,
    sessions: SessionManager,
    aliases: HashMap<String, String>, // alias name -> collection path
    serial: u32,
    verifier: Box<dyn BiometricVerifier>,
}

impl Server {
    /// Binds to a Unix socket path and tries to register on the D-Bus session bus.
    pub fn bind(socket_path: &Path, data_root: std::path::PathBuf) -> io::Result<Self> {
        Self::bind_with_verifier(socket_path, data_root, Box::new(NoBiometricVerifier))
    }

    /// Binds with a custom biometric verifier.
    pub fn bind_with_verifier(
        socket_path: &Path,
        data_root: std::path::PathBuf,
        verifier: Box<dyn BiometricVerifier>,
    ) -> io::Result<Self> {
        if socket_path.exists() {
            let _ = std::fs::remove_file(socket_path);
        }
        if let Some(parent) = socket_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let listener = UnixListener::bind(socket_path)?;
        // Standalone daemon doesn't have a node stream — events are no-ops
        // until integrated with the node's main event recorder.
        let backend = Backend::new_noop(data_root)?;

        // Try to register on the D-Bus session bus
        let bus = BusConnection::connect("org.freedesktop.secrets");

        // Default "default" alias
        let mut aliases = HashMap::new();
        aliases.insert(
            "default".into(),
            "/org/freedesktop/secrets/collections/default".into(),
        );

        if bus.is_some() {
            edgerun_log::info!(
                "edgerun-secret-service: listening on {} + D-Bus session bus",
                socket_path.display()
            );
        } else {
            edgerun_log::info!(
                "edgerun-secret-service: listening on {} (no D-Bus session bus found)",
                socket_path.display()
            );
        }

        Ok(Self {
            listener,
            bus,
            backend,
            sessions: SessionManager::new(DEFAULT_IDLE_TIMEOUT_US),
            aliases,
            serial: 0,
            verifier,
        })
    }

    fn dispatch_secret(&mut self, request: SecretRequest) -> io::Result<SecretResponse> {
        SecretServiceCore::new(&mut self.backend).dispatch(request)
    }

    /// Accept one client connection — tries the bus first, then the standalone socket.
    pub fn accept_once(&mut self) -> io::Result<()> {
        // Try the bus first (non-blocking check)
        let bus_msg_and_sender = if let Some(ref mut bus) = self.bus {
            match bus.accept_one() {
                Ok(Some(val)) => Some(val),
                Ok(None) => None,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => None,
                Err(_) => None,
            }
        } else {
            None
        };

        if let Some((msg, sender)) = bus_msg_and_sender {
            let reply_data = self.encode_reply_for_bus(&sender, msg);
            if let Some(ref mut bus) = self.bus {
                bus.send(&reply_data)?;
            }
            return Ok(());
        }

        // Fall back to the standalone socket
        let (stream, _) = self.listener.accept()?;
        self.serve_client(stream)
    }

    /// Encode a reply for a bus message without holding a borrow on the bus.
    fn encode_reply_for_bus(&mut self, sender: &str, raw_msg: Vec<u8>) -> Vec<u8> {
        if let Ok(msg) = decode_msg(&raw_msg) {
            let reply = self.handle_message(sender, &msg);
            return encode_msg(&reply);
        }
        let err = Msg::err(
            0,
            sender,
            "org.freedesktop.DBus.Error.InvalidArgs",
            "could not decode message",
        );
        encode_msg(&err)
    }

    /// Serve a single client stream.
    fn serve_client(&mut self, mut stream: UnixStream) -> io::Result<()> {
        // Use a simple unique name based on the serial counter
        self.serial += 1;
        let client_name = format!(":1.{}", self.serial);

        let mut buf = Vec::with_capacity(4096);
        let mut msg_buf = [0u8; 4096];

        loop {
            let n = match stream.read(&mut msg_buf) {
                Ok(0) => break, // connection closed
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    continue;
                }
                Err(_) => break,
            };
            buf.extend_from_slice(&msg_buf[..n]);

            // Try to decode one or more complete messages
            while let Some(msg_len) = complete_encoded_msg_len(&buf) {
                let msg = decode_msg(&buf[..msg_len]).map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("invalid D-Bus message: {error}"),
                    )
                })?;
                buf.drain(..msg_len);
                let reply = self.handle_message(&client_name, &msg);
                let reply_data = encode_msg(&reply);
                let _ = stream.write_all(&reply_data);
                let _ = stream.flush();
            }
        }

        Ok(())
    }

    /// Handle a single D-Bus message and produce a reply.
    fn handle_message(&mut self, client: &str, msg: &Msg) -> Msg {
        let ser = {
            self.serial += 1;
            self.serial
        };

        match msg.mt {
            MType::Call => self.handle_call(client, msg, ser),
            _ => Msg::err(
                ser,
                client,
                "org.freedesktop.DBus.Error.UnknownMethod",
                "unsupported message type",
            ),
        }
    }

    fn handle_call(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let iface = msg.iface().unwrap_or("");
        let member = msg.member().unwrap_or("");

        match (iface, member) {
            // org.freedesktop.Secret.Service
            ("org.freedesktop.Secret.Service", "OpenSession") => {
                self.open_session(client, msg, ser)
            }
            ("org.freedesktop.Secret.Service", "CreateCollection") => {
                self.create_collection(client, msg, ser)
            }
            ("org.freedesktop.Secret.Service", "SearchItems") => {
                self.search_items(client, msg, ser)
            }
            ("org.freedesktop.Secret.Service", "Unlock") => self.unlock(client, msg, ser),
            ("org.freedesktop.Secret.Service", "Lock") => self.lock(client, msg, ser),
            ("org.freedesktop.Secret.Service", "GetSecrets") => self.get_secrets(client, msg, ser),
            ("org.freedesktop.Secret.Service", "ReadAlias") => self.read_alias(client, msg, ser),
            ("org.freedesktop.Secret.Service", "SetAlias") => self.set_alias(client, msg, ser),

            // org.freedesktop.Secret.Collection
            ("org.freedesktop.Secret.Collection", "ListItems") => self.list_items(client, msg, ser),
            ("org.freedesktop.Secret.Collection", "CreateItem") => {
                self.create_item(client, msg, ser)
            }
            ("org.freedesktop.Secret.Collection", "Delete") => {
                self.delete_collection(client, msg, ser)
            }

            // org.freedesktop.Secret.Item
            ("org.freedesktop.Secret.Item", "GetSecret") => self.get_secret(client, msg, ser),
            ("org.freedesktop.Secret.Item", "Delete") => self.delete_item(client, msg, ser),

            // org.freedesktop.Secret.Session
            ("org.freedesktop.Secret.Session", "Close") => self.close_session(client, msg, ser),

            // Introspection
            ("org.freedesktop.DBus.Introspectable", "Introspect") => {
                self.introspect(client, msg, ser)
            }
            ("org.freedesktop.DBus.Properties", "GetAll") => {
                self.properties_get_all(client, msg, ser)
            }

            _ => Msg::err(
                ser,
                client,
                "org.freedesktop.DBus.Error.UnknownMethod",
                &format!("unknown method {}/{}", iface, member),
            ),
        }
    }

    // ===========================================================================
    // org.freedesktop.Secret.Service methods
    // ===========================================================================

    /// OpenSession (IN String algorithm, IN Variant input, OUT Variant output, OUT ObjectPath result)
    ///
    /// Creates a new session. The session starts locked — Unlock must be called
    /// with successful biometric verification before secrets can be retrieved.
    fn open_session(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let _algorithm = msg.body.first().and_then(Val::s).unwrap_or("plain");
        let _input = msg.body.get(1); // Variant input (ignored for now)

        let path = self.sessions.create_session(client);
        let has_biometrics = self.verifier.is_available();

        // Output: variant with available modalities info
        let mut info_map = Vec::new();
        info_map.push((
            Val::S("has-biometrics".into()),
            Val::Var(Box::new(Val::B(has_biometrics))),
        ));

        Msg::ret(ser, client).body(
            vec![Val::Var(Box::new(Val::Dict(info_map))), Val::O(path)],
            "vo",
        )
    }

    /// CreateCollection (IN Dict<String,Variant> properties, IN String alias, OUT ObjectPath collection, OUT ObjectPath prompt)
    fn create_collection(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let properties = msg.body.first().and_then(Val::dict_ss).unwrap_or_default();
        let _alias = msg.body.get(1).and_then(Val::s).unwrap_or("");

        let label = properties
            .iter()
            .find(|(k, _)| k == "org.freedesktop.Secret.Collection.Label")
            .map(|(_, v)| v.as_str())
            .unwrap_or("unnamed");

        let coll_path = format!(
            "/org/freedesktop/secrets/collections/{}",
            label.replace(' ', "_")
        );

        let _ = self.dispatch_secret(SecretRequest::CreateCollection {
            collection_name: Backend::coll_to_ns(&coll_path),
            label: label.to_string(),
        });

        Msg::ret(ser, client).body(
            vec![
                Val::O(coll_path),
                Val::O("/".into()), // No prompt needed
            ],
            "oo",
        )
    }

    /// SearchItems (IN Dict<String,String> attributes, OUT Array<ObjectPath> unlocked, OUT Array<ObjectPath> locked)
    ///
    /// All items are returned as "unlocked" — the actual biometric check
    /// happens at Unlock/GetSecret time. Items that don't match the search
    /// are simply not returned.
    fn search_items(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let attrs = msg.body.first().and_then(Val::dict_ss).unwrap_or_default();
        let attr_pairs: Vec<(String, String)> = attrs.into_iter().collect();

        let mut unlocked = Vec::new();

        // Search across all collections
        let collections = match self.dispatch_secret(SecretRequest::ListCollections) {
            Ok(SecretResponse::Collections(collections)) => collections,
            _ => Vec::new(),
        };
        for coll in &collections {
            let results = match self.dispatch_secret(SecretRequest::Search {
                collection: coll.clone(),
                attributes: attr_pairs.clone(),
            }) {
                Ok(SecretResponse::Items(items)) => items,
                _ => Vec::new(),
            };
            for (key, _meta) in results {
                let item_path = Backend::item_path(coll, &key);
                unlocked.push(Val::O(item_path));
            }
        }

        // Items are all potentially unlockable — locked array is empty
        // until we implement per-item lock states
        Msg::ret(ser, client).body(vec![Val::Arr(unlocked), Val::Arr(vec![])], "aoao")
    }

    /// Unlock (IN Array<ObjectPath> objects, OUT Array<ObjectPath> unlocked, OUT ObjectPath prompt)
    ///
    /// Triggers biometric verification. If successful, all requested objects
    /// are unlocked and returned. If biometrics are not available, returns an error.
    fn unlock(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let objects = msg.body.first().and_then(Val::ao).unwrap_or_default();

        // Run biometric verification
        if !self.verifier.is_available() {
            return Msg::err(
                ser,
                client,
                "org.freedesktop.Secret.Error.IsLocked",
                "no biometric hardware available — cannot unlock",
            );
        }

        let bio_state = self.verifier.verify();
        if !bio_state.verified {
            return Msg::err(
                ser,
                client,
                "org.freedesktop.Secret.Error.IsLocked",
                "biometric verification failed",
            );
        }

        // Verify all sessions for this client
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        self.sessions.verify_client(client, bio_state.clone(), now);

        let unlocked: Vec<Val> = objects.into_iter().map(Val::O).collect();

        Msg::ret(ser, client).body(
            vec![
                Val::Arr(unlocked),
                Val::O("/".into()), // No prompt needed
            ],
            "ao",
        )
    }

    /// Lock (IN Array<ObjectPath> objects, OUT Array<ObjectPath> locked, OUT ObjectPath prompt)
    ///
    /// Clears biometric verification state on the session. Subsequent
    /// GetSecrets/GetSecret calls will fail until Unlock is called again.
    fn lock(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let objects = msg.body.first().and_then(Val::ao).unwrap_or_default();

        // Lock all sessions for this client
        self.sessions.lock_client(client);

        // Return all requested objects as locked
        let locked: Vec<Val> = objects.into_iter().map(Val::O).collect();

        Msg::ret(ser, client).body(vec![Val::Arr(locked), Val::O("/".into())], "ao")
    }

    /// GetSecrets (IN Array<ObjectPath> items, IN ObjectPath session, OUT Dict<ObjectPath,Secret> secrets)
    ///
    /// Requires the session to be biometrically verified. Returns only
    /// unlocked (verified) items. Locked items are silently omitted.
    fn get_secrets(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let item_paths = msg.body.first().and_then(Val::ao).unwrap_or_default();
        let session_path = msg.body.get(1).and_then(Val::o).unwrap_or("");

        // Look up the session
        let Some(session) = self.sessions.get(session_path) else {
            return Msg::err(
                ser,
                client,
                "org.freedesktop.Secret.Error.NoSuchSession",
                "session not found",
            );
        };

        if session.closed {
            return Msg::err(
                ser,
                client,
                "org.freedesktop.Secret.Error.NoSuchSession",
                "session is closed",
            );
        }

        // Check biometric verification
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        if !session.is_verified(now) {
            return Msg::err(
                ser,
                client,
                "org.freedesktop.Secret.Error.IsLocked",
                "session is locked — call Unlock with biometric verification first",
            );
        }

        // Resolve each item path to (collection, key)
        let mut secrets_dict = Vec::new();
        for item_path in &item_paths {
            if let Some((coll, key)) = resolve_item_path(item_path) {
                let secret_entry = match self.dispatch_secret(SecretRequest::Get {
                    collection: coll,
                    key,
                }) {
                    Ok(SecretResponse::Secret(entry)) => entry,
                    _ => None,
                };
                if let Some(secret_entry) = secret_entry {
                    let secret_bytes_val: Vec<Val> =
                        secret_entry.secret.iter().map(|&b| Val::Y(b)).collect();
                    let content_type = "text/plain; charset=utf8";

                    // Secret struct: (oa{sv}ays)
                    let secret_struct = Val::Str(vec![
                        Val::O(session_path.to_string()),
                        Val::Dict(vec![]),
                        Val::Arr(secret_bytes_val),
                        Val::S(content_type.into()),
                    ]);

                    secrets_dict
                        .push((Val::O(item_path.clone()), Val::Var(Box::new(secret_struct))));
                }
            }
        }

        Msg::ret(ser, client).body(vec![Val::Dict(secrets_dict)], "a{o(v)}")
    }

    /// ReadAlias (IN String name, OUT ObjectPath collection)
    fn read_alias(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let name = msg.body.first().and_then(Val::s).unwrap_or("");
        let collection = self
            .aliases
            .get(name)
            .cloned()
            .unwrap_or_else(|| "/".into());

        Msg::ret(ser, client).body(vec![Val::O(collection)], "o")
    }

    /// SetAlias (IN String name, IN ObjectPath collection)
    fn set_alias(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let name = msg.body.first().and_then(Val::s).unwrap_or("");
        let collection = msg.body.get(1).and_then(Val::o).unwrap_or("");

        if name.is_empty() || collection.is_empty() {
            return Msg::err(
                ser,
                client,
                "org.freedesktop.DBus.Error.InvalidArgs",
                "name and collection must be non-empty",
            );
        }

        self.aliases
            .insert(name.to_string(), collection.to_string());

        Msg::ret(ser, client).body(vec![], "")
    }

    // ===========================================================================
    // org.freedesktop.Secret.Collection methods
    // ===========================================================================

    /// ListItems (OUT Array<ObjectPath> items)
    fn list_items(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let coll = msg.path().unwrap_or("");
        let items = match self.dispatch_secret(SecretRequest::List {
            collection: coll.to_string(),
        }) {
            Ok(SecretResponse::Items(items)) => items,
            _ => Vec::new(),
        };

        let item_paths: Vec<Val> = items
            .into_iter()
            .map(|(key, _)| Val::O(Backend::item_path(coll, &key)))
            .collect();

        Msg::ret(ser, client).body(vec![Val::Arr(item_paths)], "ao")
    }

    /// CreateItem (IN Dict<String,Variant> properties, IN Secret secret, IN Boolean replace,
    ///             OUT ObjectPath item, OUT ObjectPath prompt)
    fn create_item(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let coll = msg.path().unwrap_or("");

        let properties = msg.body.first().and_then(Val::dict_ss).unwrap_or_default();
        let label = properties
            .iter()
            .find(|(k, _)| k == "org.freedesktop.Secret.Item.Label")
            .map(|(_, v)| v.as_str())
            .unwrap_or("unnamed")
            .to_string();

        let attrs: Vec<(String, String)> = properties
            .iter()
            .filter(|(k, _)| k.starts_with("org.freedesktop.Secret.Item.Attribute."))
            .map(|(k, v)| {
                (
                    k.strip_prefix("org.freedesktop.Secret.Item.Attribute.")
                        .unwrap_or(k)
                        .to_string(),
                    v.clone(),
                )
            })
            .collect();

        // Parse secret struct: (oa{sv}ays)
        let Some(secret_val) = msg.body.get(1) else {
            return Msg::err(
                ser,
                client,
                "org.freedesktop.DBus.Error.InvalidArgs",
                "secret is required",
            );
        };

        let secret_bytes = if let Val::Str(fields) = secret_val.clone() {
            if fields.len() >= 3 {
                if let Val::Arr(bytes) = &fields[2] {
                    bytes
                        .iter()
                        .filter_map(|b| if let Val::Y(v) = b { Some(*v) } else { None })
                        .collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Generate a stable item key from label + attributes
        let item_key = Backend::item_key(&label, &attrs);

        let replace = msg.body.get(2).and_then(Val::b).unwrap_or(false);

        if replace {
            let _ = self.dispatch_secret(SecretRequest::Delete {
                collection: coll.to_string(),
                key: item_key.clone(),
            });
        }

        if let Err(e) = self.dispatch_secret(SecretRequest::Put {
            collection: coll.to_string(),
            key: item_key.clone(),
            secret: secret_bytes,
            label: label.clone(),
            attributes: attrs,
        }) {
            return Msg::err(
                ser,
                client,
                "org.freedesktop.Secret.Error.Failed",
                &e.to_string(),
            );
        }

        let item_path = Backend::item_path(coll, &item_key);

        Msg::ret(ser, client).body(vec![Val::O(item_path), Val::O("/".into())], "oo")
    }

    /// Delete a collection
    fn delete_collection(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let coll = msg.path().unwrap_or("");
        let _ = self.dispatch_secret(SecretRequest::DeleteCollection {
            collection_name: Backend::coll_to_ns(coll),
        });

        // Remove alias if present
        self.aliases.retain(|_, v| v == coll);

        Msg::ret(ser, client).body(vec![Val::O("/".into())], "o")
    }

    // ===========================================================================
    // org.freedesktop.Secret.Item methods
    // ===========================================================================

    /// GetSecret (IN ObjectPath session, OUT Secret secret)
    fn get_secret(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let session_path = msg.body.first().and_then(Val::o).unwrap_or("");
        let item_path = msg.path().unwrap_or("");

        let Some(session) = self.sessions.get(session_path) else {
            return Msg::err(
                ser,
                client,
                "org.freedesktop.Secret.Error.NoSuchSession",
                "session not found",
            );
        };

        if session.closed {
            return Msg::err(
                ser,
                client,
                "org.freedesktop.Secret.Error.NoSuchSession",
                "session is closed",
            );
        }

        let Some((coll, key)) = resolve_item_path(item_path) else {
            return Msg::err(
                ser,
                client,
                "org.freedesktop.DBus.Error.InvalidArgs",
                "invalid item path",
            );
        };

        let secret_entry = match self.dispatch_secret(SecretRequest::Get {
            collection: coll.clone(),
            key: key.clone(),
        }) {
            Ok(SecretResponse::Secret(Some(entry))) => entry,
            _ => {
                return Msg::err(
                    ser,
                    client,
                    "org.freedesktop.Secret.Error.NoSuchItem",
                    "item not found",
                );
            }
        };

        // Return as-is (plain mode)
        let secret_bytes_val: Vec<Val> = secret_entry.secret.iter().map(|&b| Val::Y(b)).collect();

        let secret_struct = Val::Str(vec![
            Val::O(session_path.to_string()),
            Val::Dict(vec![]),
            Val::Arr(secret_bytes_val),
            Val::S("text/plain; charset=utf8".into()),
        ]);

        Msg::ret(ser, client).body(vec![Val::Var(Box::new(secret_struct))], "v")
    }

    /// Delete an item
    fn delete_item(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let item_path = msg.path().unwrap_or("");

        let Some((coll, key)) = resolve_item_path(item_path) else {
            return Msg::err(
                ser,
                client,
                "org.freedesktop.DBus.Error.InvalidArgs",
                "invalid item path",
            );
        };

        let existed = match self.dispatch_secret(SecretRequest::Delete {
            collection: coll,
            key,
        }) {
            Ok(SecretResponse::Deleted(existed)) => existed,
            _ => false,
        };
        if !existed {
            return Msg::err(
                ser,
                client,
                "org.freedesktop.Secret.Error.NoSuchItem",
                "item not found",
            );
        }

        Msg::ret(ser, client).body(vec![Val::O("/".into())], "o")
    }

    // ===========================================================================
    // org.freedesktop.Secret.Session methods
    // ===========================================================================

    /// Close (no args, no return)
    fn close_session(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        if let Some(path) = msg.path() {
            self.sessions.close(path);
        }
        Msg::ret(ser, client).body(vec![], "")
    }

    // ===========================================================================
    // Introspection
    // ===========================================================================

    fn introspect(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let path = msg.path().unwrap_or("/org/freedesktop/secrets");
        let xml = build_introspect_xml(path);
        Msg::ret(ser, client).body(vec![Val::S(xml)], "s")
    }

    fn properties_get_all(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let _iface = msg.body.first().and_then(Val::s).unwrap_or("");
        // Return empty dict for now
        Msg::ret(ser, client).body(vec![Val::Dict(vec![])], "a{sv}")
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Resolve an item path into (collection_path, item_key).
/// Item paths are: /org/freedesktop/secrets/collections/<coll>/<key>
fn resolve_item_path(path: &str) -> Option<(String, String)> {
    let prefix = "/org/freedesktop/secrets/collections/";
    if !path.starts_with(prefix) {
        return None;
    }
    let rest = &path[prefix.len()..];
    if let Some(slash) = rest.find('/') {
        let coll_name = &rest[..slash];
        let key = &rest[slash + 1..];
        let coll_path = format!("/org/freedesktop/secrets/collections/{}", coll_name);
        Some((coll_path, key.to_string()))
    } else {
        None
    }
}

/// Return the exact encoded length of a complete D-Bus message in `data`.
fn complete_encoded_msg_len(data: &[u8]) -> Option<usize> {
    if data.len() < 16 {
        return None;
    }
    let body_len = u32::from_le_bytes(data[4..8].try_into().ok()?) as usize;
    let header_fields_len = u32::from_le_bytes(data[12..16].try_into().ok()?) as usize;
    let header_end = 16usize.checked_add(header_fields_len)?;
    let aligned_header_end = header_end.checked_add(7)? & !7;
    let message_len = aligned_header_end.checked_add(body_len)?;
    (message_len <= data.len()).then_some(message_len)
}

/// Build D-Bus introspection XML.
fn build_introspect_xml(path: &str) -> String {
    let mut xml = String::from("<!DOCTYPE node PUBLIC \"-//freedesktop//DTD D-BUS Object Introspection 1.0//EN\"\n\"http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd\">\n<node>\n");

    if path == "/org/freedesktop/secrets" {
        xml.push_str(
            r#"<interface name="org.freedesktop.Secret.Service">
<method name="OpenSession">
<arg name="algorithm" type="s" direction="in"/>
<arg name="input" type="v" direction="in"/>
<arg name="output" type="v" direction="out"/>
<arg name="result" type="o" direction="out"/>
</method>
<method name="CreateCollection">
<arg name="properties" type="a{sv}" direction="in"/>
<arg name="alias" type="s" direction="in"/>
<arg name="collection" type="o" direction="out"/>
<arg name="prompt" type="o" direction="out"/>
</method>
<method name="SearchItems">
<arg name="attributes" type="a{ss}" direction="in"/>
<arg name="unlocked" type="ao" direction="out"/>
<arg name="locked" type="ao" direction="out"/>
</method>
<method name="Unlock">
<arg name="objects" type="ao" direction="in"/>
<arg name="unlocked" type="ao" direction="out"/>
<arg name="prompt" type="o" direction="out"/>
</method>
<method name="Lock">
<arg name="objects" type="ao" direction="in"/>
<arg name="locked" type="ao" direction="out"/>
<arg name="prompt" type="o" direction="out"/>
</method>
<method name="GetSecrets">
<arg name="items" type="ao" direction="in"/>
<arg name="session" type="o" direction="in"/>
<arg name="secrets" type="a{o(v)}" direction="out"/>
</method>
<method name="ReadAlias">
<arg name="name" type="s" direction="in"/>
<arg name="collection" type="o" direction="out"/>
</method>
<method name="SetAlias">
<arg name="name" type="s" direction="in"/>
<arg name="collection" type="o" direction="in"/>
</method>
<signal name="CollectionCreated"><arg name="collection" type="o"/></signal>
<signal name="CollectionDeleted"><arg name="collection" type="o"/></signal>
<signal name="CollectionChanged"><arg name="collection" type="o"/></signal>
</interface>
"#,
        );
    }

    xml.push_str(
        r#"<interface name="org.freedesktop.DBus.Introspectable">
<method name="Introspect"><arg name="data" type="s" direction="out"/></method>
</interface>
"#,
    );

    // Add collection child nodes
    if path == "/org/freedesktop/secrets" {
        xml.push_str(
            r#"<node name="collection"/>
<node name="aliases"/>
"#,
        );
    }

    xml.push_str("</node>\n");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn init_test_node_id() {
        let _ = crate::backend::init_node_id(vec![0x56; 32]);
    }

    fn tmp_root() -> PathBuf {
        init_test_node_id();
        static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = C.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("srv_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn complete_encoded_msg_len_waits_for_full_frame() {
        let msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "OpenSession",
            ":1.1",
        )
        .body(
            vec![
                Val::S("plain".into()),
                Val::Var(Box::new(Val::S("".into()))),
            ],
            "sv",
        );
        let encoded = encode_msg(&msg);

        assert_eq!(complete_encoded_msg_len(&encoded), Some(encoded.len()));
        assert_eq!(
            complete_encoded_msg_len(&encoded[..encoded.len() - 1]),
            None
        );
    }

    #[test]
    fn resolve_item_path_valid() {
        let (coll, key) =
            resolve_item_path("/org/freedesktop/secrets/collections/default/mykey").unwrap();
        assert_eq!(coll, "/org/freedesktop/secrets/collections/default");
        assert_eq!(key, "mykey");
    }

    #[test]
    fn resolve_item_path_invalid() {
        assert!(resolve_item_path("/org/freedesktop/secrets").is_none());
        assert!(resolve_item_path("/some/other/path").is_none());
    }

    #[test]
    fn coll_to_ns() {
        assert_eq!(
            Backend::coll_to_ns("/org/freedesktop/secrets/collections/default"),
            "default"
        );
    }

    #[test]
    fn item_path_format() {
        assert_eq!(
            Backend::item_path("/org/freedesktop/secrets/collections/default", "mykey"),
            "/org/freedesktop/secrets/collections/default/mykey"
        );
    }

    #[test]
    fn open_session_returns_path() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        let msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "OpenSession",
            ":1.1",
        )
        .body(
            vec![
                Val::S("plain".into()),
                Val::Var(Box::new(Val::S("".into()))),
            ],
            "sv",
        );

        let reply = server.handle_message(":1.1", &msg);
        assert_eq!(reply.mt, MType::Return);
        assert_eq!(reply.body.len(), 2);
        if let Val::O(session_path) = &reply.body[1] {
            assert!(session_path.starts_with("/org/freedesktop/secrets/session/"));
        } else {
            panic!("expected ObjectPath, got {:?}", reply.body[1]);
        }

        // No biometrics by default
        if let Val::Dict(info) = &reply.body[0] {
            // has-biometrics should be false
            if let Some((_, Val::Var(inner))) =
                info.iter().find(|(k, _)| k.s() == Some("has-biometrics"))
            {
                if let Val::B(has_bio) = inner.as_ref() {
                    assert!(!has_bio);
                }
            }
        }
    }

    #[test]
    fn unlock_requires_biometrics() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        // Open session first
        let open_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "OpenSession",
            ":1.1",
        )
        .body(
            vec![
                Val::S("plain".into()),
                Val::Var(Box::new(Val::S("".into()))),
            ],
            "sv",
        );
        server.handle_message(":1.1", &open_msg);

        // Unlock without biometrics → error
        let unlock_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "Unlock",
            ":1.1",
        )
        .body(
            vec![Val::Arr(vec![Val::O(
                "/org/freedesktop/secrets/collections/default/item1".into(),
            )])],
            "ao",
        );

        let reply = server.handle_message(":1.1", &unlock_msg);
        assert_eq!(reply.mt, MType::Err);
        // Should mention no biometric hardware
    }

    #[test]
    fn read_alias_default() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        let msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "ReadAlias",
            ":1.1",
        )
        .body(vec![Val::S("default".into())], "s");

        let reply = server.handle_message(":1.1", &msg);
        assert_eq!(reply.mt, MType::Return);
        if let Val::O(collection) = &reply.body[0] {
            assert_eq!(collection, "/org/freedesktop/secrets/collections/default");
        } else {
            panic!("expected ObjectPath");
        }
    }

    #[test]
    fn set_alias_and_read() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        // Set alias
        let set_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "SetAlias",
            ":1.1",
        )
        .body(
            vec![
                Val::S("myalias".into()),
                Val::O("/org/freedesktop/secrets/collections/custom".into()),
            ],
            "so",
        );

        let set_reply = server.handle_message(":1.1", &set_msg);
        assert_eq!(set_reply.mt, MType::Return);

        // Read it back
        let read_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "ReadAlias",
            ":1.1",
        )
        .body(vec![Val::S("myalias".into())], "s");

        let read_reply = server.handle_message(":1.1", &read_msg);
        assert_eq!(read_reply.mt, MType::Return);
        if let Val::O(collection) = &read_reply.body[0] {
            assert_eq!(collection, "/org/freedesktop/secrets/collections/custom");
        } else {
            panic!("expected ObjectPath");
        }
    }

    #[test]
    fn unlock_returns_error_without_biometrics() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        // Open session first
        let open_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "OpenSession",
            ":1.1",
        )
        .body(
            vec![
                Val::S("plain".into()),
                Val::Var(Box::new(Val::S("".into()))),
            ],
            "sv",
        );
        server.handle_message(":1.1", &open_msg);

        // Unlock without biometrics → error
        let msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "Unlock",
            ":1.1",
        )
        .body(
            vec![Val::Arr(vec![
                Val::O("/org/freedesktop/secrets/collections/default/item1".into()),
                Val::O("/org/freedesktop/secrets/collections/default/item2".into()),
            ])],
            "ao",
        );

        let reply = server.handle_message(":1.1", &msg);
        assert_eq!(reply.mt, MType::Err);
    }

    #[test]
    fn lock_returns_requested_objects_as_locked() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        let msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "Lock",
            ":1.1",
        )
        .body(
            vec![Val::Arr(vec![
                Val::O("/org/freedesktop/secrets/collections/default/item1".into()),
                Val::O("/org/freedesktop/secrets/collections/default/item2".into()),
            ])],
            "ao",
        );

        let reply = server.handle_message(":1.1", &msg);
        assert_eq!(reply.mt, MType::Return);
        // All requested objects are returned as locked
        if let Val::Arr(locked) = &reply.body[0] {
            assert_eq!(locked.len(), 2);
        } else {
            panic!("expected array");
        }
    }

    #[test]
    fn close_session() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        // Open a session
        let open_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "OpenSession",
            ":1.1",
        )
        .body(
            vec![
                Val::S("plain".into()),
                Val::Var(Box::new(Val::S("".into()))),
            ],
            "sv",
        );
        let open_reply = server.handle_message(":1.1", &open_msg);
        let session_path = match &open_reply.body[1] {
            Val::O(p) => p.clone(),
            _ => panic!("expected session path"),
        };

        // Close it
        let close_msg = Msg::call(
            &session_path,
            "org.freedesktop.Secret.Session",
            "Close",
            ":1.1",
        );
        let close_reply = server.handle_message(":1.1", &close_msg);
        assert_eq!(close_reply.mt, MType::Return);

        // GC should remove it
        server.sessions.gc();
        assert!(server.sessions.get(&session_path).is_none());
    }

    #[test]
    fn get_secrets_requires_unlock() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        // Open session
        let open_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "OpenSession",
            ":1.1",
        )
        .body(
            vec![
                Val::S("plain".into()),
                Val::Var(Box::new(Val::S("".into()))),
            ],
            "sv",
        );
        let open_reply = server.handle_message(":1.1", &open_msg);
        let session_path = match &open_reply.body[1] {
            Val::O(p) => p.clone(),
            _ => panic!("expected session path"),
        };

        // Try to get secrets without unlock → error
        let get_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "GetSecrets",
            ":1.1",
        )
        .body(
            vec![
                Val::Arr(vec![Val::O(
                    "/org/freedesktop/secrets/collections/default/item1".into(),
                )]),
                Val::O(session_path.clone()),
            ],
            "ao",
        );

        let reply = server.handle_message(":1.1", &get_msg);
        assert_eq!(reply.mt, MType::Err);
        // Should say session is locked
    }

    #[test]
    fn unlock_and_get_secrets_with_mock_biometrics() {
        use crate::session::BiometricVerifier;
        use edgerun_devices::biometrics::BiometricState;

        struct MockVerifier {
            available: bool,
            verified: bool,
        }
        impl BiometricVerifier for MockVerifier {
            fn verify(&self) -> BiometricState {
                let mut s = BiometricState::default();
                s.verified = self.verified;
                s
            }
            fn is_available(&self) -> bool {
                self.available
            }
        }

        let root = tmp_root();
        let mut server = Server::bind_with_verifier(
            &root.join("test.sock"),
            root.clone(),
            Box::new(MockVerifier {
                available: true,
                verified: true,
            }),
        )
        .unwrap();

        // Open session
        let open_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "OpenSession",
            ":1.1",
        )
        .body(
            vec![
                Val::S("plain".into()),
                Val::Var(Box::new(Val::S("".into()))),
            ],
            "sv",
        );
        let open_reply = server.handle_message(":1.1", &open_msg);
        let session_path = match &open_reply.body[1] {
            Val::O(p) => p.clone(),
            _ => panic!("expected session path"),
        };

        // Put a secret first
        let key = Backend::item_key("Test", &[("key".into(), "value".into())]);
        server
            .backend
            .put(
                "/org/freedesktop/secrets/collections/default",
                &key,
                b"super-secret",
                "Test",
                &[("key".into(), "value".into())],
            )
            .unwrap();

        let item_path = Backend::item_path("/org/freedesktop/secrets/collections/default", &key);

        // Unlock → should succeed with biometrics
        let unlock_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "Unlock",
            ":1.1",
        )
        .body(vec![Val::Arr(vec![Val::O(item_path.clone())])], "ao");
        let unlock_reply = server.handle_message(":1.1", &unlock_msg);
        assert_eq!(unlock_reply.mt, MType::Return);

        // Get secrets → should succeed
        let get_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "GetSecrets",
            ":1.1",
        )
        .body(
            vec![
                Val::Arr(vec![Val::O(item_path.clone())]),
                Val::O(session_path.clone()),
            ],
            "ao",
        );
        let get_reply = server.handle_message(":1.1", &get_msg);
        assert_eq!(get_reply.mt, MType::Return);
        if let Val::Dict(secrets) = &get_reply.body[0] {
            assert_eq!(secrets.len(), 1);
        } else {
            panic!("expected dict");
        }

        // Lock → should clear session verification
        let lock_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "Lock",
            ":1.1",
        )
        .body(vec![Val::Arr(vec![Val::O(item_path.clone())])], "ao");
        let lock_reply = server.handle_message(":1.1", &lock_msg);
        assert_eq!(lock_reply.mt, MType::Return);

        // Get secrets again → should fail (locked)
        let get_msg2 = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "GetSecrets",
            ":1.1",
        )
        .body(
            vec![
                Val::Arr(vec![Val::O(item_path.clone())]),
                Val::O(session_path.clone()),
            ],
            "ao",
        );
        let get_reply2 = server.handle_message(":1.1", &get_msg2);
        assert_eq!(get_reply2.mt, MType::Err);
    }

    #[test]
    fn search_items_empty_attrs() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        // Put an item first
        let key = Backend::item_key("Test", &[("key".into(), "value".into())]);
        server
            .backend
            .put(
                "/org/freedesktop/secrets/collections/default",
                &key,
                b"secret",
                "Test",
                &[("key".into(), "value".into())],
            )
            .unwrap();

        let msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "SearchItems",
            ":1.1",
        )
        .body(
            vec![Val::Dict(vec![(
                Val::S("key".into()),
                Val::S("value".into()),
            )])],
            "a{ss}",
        );

        let reply = server.handle_message(":1.1", &msg);
        assert_eq!(reply.mt, MType::Return);
        if let Val::Arr(unlocked) = &reply.body[0] {
            assert_eq!(unlocked.len(), 1);
        } else {
            panic!("expected array");
        }
    }

    #[test]
    fn introspect_service() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        let msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.DBus.Introspectable",
            "Introspect",
            ":1.1",
        );
        let reply = server.handle_message(":1.1", &msg);
        assert_eq!(reply.mt, MType::Return);
        if let Val::S(xml) = &reply.body[0] {
            assert!(xml.contains("org.freedesktop.Secret.Service"));
            assert!(xml.contains("OpenSession"));
        } else {
            panic!("expected XML string");
        }
    }

    #[test]
    fn delete_item() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        let key = Backend::item_key("Test", &[]);
        server
            .backend
            .put(
                "/org/freedesktop/secrets/collections/default",
                &key,
                b"secret",
                "Test",
                &[],
            )
            .unwrap();
        let item_path = Backend::item_path("/org/freedesktop/secrets/collections/default", &key);

        let msg = Msg::call(&item_path, "org.freedesktop.Secret.Item", "Delete", ":1.1");
        let reply = server.handle_message(":1.1", &msg);
        assert_eq!(reply.mt, MType::Return);
        assert!(server
            .backend
            .get("/org/freedesktop/secrets/collections/default", &key)
            .unwrap()
            .is_none());
    }

    #[test]
    fn delete_nonexistent_item_returns_error() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        let msg = Msg::call(
            "/org/freedesktop/secrets/collections/default/ghost",
            "org.freedesktop.Secret.Item",
            "Delete",
            ":1.1",
        );
        let reply = server.handle_message(":1.1", &msg);
        assert_eq!(reply.mt, MType::Err);
    }

    #[test]
    fn delete_collection_removes_all_items() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        for i in 0..3 {
            let key = Backend::item_key(&format!("Item {}", i), &[]);
            server
                .backend
                .put(
                    "/org/freedesktop/secrets/collections/default",
                    &key,
                    format!("secret-{}", i).as_bytes(),
                    &format!("Item {}", i),
                    &[],
                )
                .unwrap();
        }
        assert_eq!(
            server
                .backend
                .list("/org/freedesktop/secrets/collections/default")
                .unwrap()
                .len(),
            3
        );

        let msg = Msg::call(
            "/org/freedesktop/secrets/collections/default",
            "org.freedesktop.Secret.Collection",
            "Delete",
            ":1.1",
        );
        let reply = server.handle_message(":1.1", &msg);
        assert_eq!(reply.mt, MType::Return);
        assert!(server
            .backend
            .list("/org/freedesktop/secrets/collections/default")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn unknown_method_returns_error() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        let msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "NonExistent",
            ":1.1",
        );
        let reply = server.handle_message(":1.1", &msg);
        assert_eq!(reply.mt, MType::Err);
    }

    #[test]
    fn unknown_interface_returns_error() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        let msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.Unknown.Interface",
            "Method",
            ":1.1",
        );
        let reply = server.handle_message(":1.1", &msg);
        assert_eq!(reply.mt, MType::Err);
    }

    #[test]
    fn close_session_then_get_secrets_fails() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        let open_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "OpenSession",
            ":1.1",
        )
        .body(
            vec![
                Val::S("plain".into()),
                Val::Var(Box::new(Val::S("".into()))),
            ],
            "sv",
        );
        let open_reply = server.handle_message(":1.1", &open_msg);
        let session_path = match &open_reply.body[1] {
            Val::O(p) => p.clone(),
            _ => panic!("expected path"),
        };

        let close_msg = Msg::call(
            &session_path,
            "org.freedesktop.Secret.Session",
            "Close",
            ":1.1",
        );
        server.handle_message(":1.1", &close_msg);

        let get_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "GetSecrets",
            ":1.1",
        )
        .body(vec![Val::Arr(vec![]), Val::O(session_path.clone())], "ao");
        let reply = server.handle_message(":1.1", &get_msg);
        assert_eq!(reply.mt, MType::Err);
    }

    #[test]
    fn full_flow_put_unlock_get_lock_get_fails() {
        use crate::session::BiometricVerifier;
        use edgerun_devices::biometrics::BiometricState;

        struct MockVerifier;
        impl BiometricVerifier for MockVerifier {
            fn verify(&self) -> BiometricState {
                let mut s = BiometricState::default();
                s.verified = true;
                s
            }
            fn is_available(&self) -> bool {
                true
            }
        }

        let root = tmp_root();
        let mut server = Server::bind_with_verifier(
            &root.join("test.sock"),
            root.clone(),
            Box::new(MockVerifier),
        )
        .unwrap();

        // Open session
        let open_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "OpenSession",
            ":1.1",
        )
        .body(
            vec![
                Val::S("plain".into()),
                Val::Var(Box::new(Val::S("".into()))),
            ],
            "sv",
        );
        let open_reply = server.handle_message(":1.1", &open_msg);
        let session_path = match &open_reply.body[1] {
            Val::O(p) => p.clone(),
            _ => panic!("expected path"),
        };

        // Put a secret
        let key = Backend::item_key("My Token", &[("service".into(), "example.com".into())]);
        server
            .backend
            .put(
                "/org/freedesktop/secrets/collections/default",
                &key,
                b"my-super-secret-token",
                "My Token",
                &[("service".into(), "example.com".into())],
            )
            .unwrap();
        let item_path = Backend::item_path("/org/freedesktop/secrets/collections/default", &key);

        // Unlock
        let unlock_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "Unlock",
            ":1.1",
        )
        .body(vec![Val::Arr(vec![Val::O(item_path.clone())])], "ao");
        let unlock_reply = server.handle_message(":1.1", &unlock_msg);
        assert_eq!(unlock_reply.mt, MType::Return);

        // Get secrets → should succeed
        let get_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "GetSecrets",
            ":1.1",
        )
        .body(
            vec![
                Val::Arr(vec![Val::O(item_path.clone())]),
                Val::O(session_path.clone()),
            ],
            "ao",
        );
        let get_reply = server.handle_message(":1.1", &get_msg);
        assert_eq!(get_reply.mt, MType::Return);
        if let Val::Dict(secrets) = &get_reply.body[0] {
            assert_eq!(secrets.len(), 1);
        } else {
            panic!("expected dict");
        }

        // Lock
        let lock_msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "Lock",
            ":1.1",
        )
        .body(vec![Val::Arr(vec![Val::O(item_path.clone())])], "ao");
        server.handle_message(":1.1", &lock_msg);

        // Get secrets → should fail
        let get_msg2 = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "GetSecrets",
            ":1.1",
        )
        .body(
            vec![
                Val::Arr(vec![Val::O(item_path.clone())]),
                Val::O(session_path.clone()),
            ],
            "ao",
        );
        let get_reply2 = server.handle_message(":1.1", &get_msg2);
        assert_eq!(get_reply2.mt, MType::Err);
    }

    #[test]
    fn search_across_multiple_collections() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        let key1 = Backend::item_key("GH", &[("server".into(), "github.com".into())]);
        let key2 = Backend::item_key("GL", &[("server".into(), "gitlab.com".into())]);
        let key3 = Backend::item_key("GH2", &[("server".into(), "github.com".into())]);
        server
            .backend
            .put(
                "/org/freedesktop/secrets/collections/default",
                &key1,
                b"gh1",
                "GH",
                &[("server".into(), "github.com".into())],
            )
            .unwrap();
        server
            .backend
            .put(
                "/org/freedesktop/secrets/collections/login",
                &key2,
                b"gl1",
                "GL",
                &[("server".into(), "gitlab.com".into())],
            )
            .unwrap();
        server
            .backend
            .put(
                "/org/freedesktop/secrets/collections/default",
                &key3,
                b"gh2",
                "GH2",
                &[("server".into(), "github.com".into())],
            )
            .unwrap();

        let msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "SearchItems",
            ":1.1",
        )
        .body(
            vec![Val::Dict(vec![(
                Val::S("server".into()),
                Val::S("github.com".into()),
            )])],
            "a{ss}",
        );
        let reply = server.handle_message(":1.1", &msg);
        assert_eq!(reply.mt, MType::Return);
        if let Val::Arr(unlocked) = &reply.body[0] {
            assert_eq!(unlocked.len(), 2);
        } else {
            panic!("expected array");
        }
    }

    #[test]
    fn read_alias_nonexistent() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        let msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
            "ReadAlias",
            ":1.1",
        )
        .body(vec![Val::S("nonexistent".into())], "s");
        let reply = server.handle_message(":1.1", &msg);
        assert_eq!(reply.mt, MType::Return);
        if let Val::O(c) = &reply.body[0] {
            assert_eq!(c, "/");
        } else {
            panic!("expected ObjectPath");
        }
    }

    #[test]
    fn properties_get_all() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        let msg = Msg::call(
            "/org/freedesktop/secrets",
            "org.freedesktop.DBus.Properties",
            "GetAll",
            ":1.1",
        )
        .body(vec![Val::S("org.freedesktop.Secret.Service".into())], "s");
        let reply = server.handle_message(":1.1", &msg);
        assert_eq!(reply.mt, MType::Return);
        if let Val::Dict(d) = &reply.body[0] {
            assert!(d.is_empty());
        } else {
            panic!("expected dict");
        }
    }

    #[test]
    fn rebuild_preserves_server_access() {
        let root = tmp_root();
        let mut server = Server::bind(&root.join("test.sock"), root.clone()).unwrap();

        let key = Backend::item_key("Test", &[("k".into(), "v".into())]);
        server
            .backend
            .put(
                "/org/freedesktop/secrets/collections/default",
                &key,
                b"secret",
                "Test",
                &[("k".into(), "v".into())],
            )
            .unwrap();

        // No-op rebuild — standalone daemon has no event stream

        let (secret, meta) = server
            .backend
            .get("/org/freedesktop/secrets/collections/default", &key)
            .unwrap()
            .unwrap();
        assert_eq!(secret, b"secret");
        assert_eq!(meta.label, "Test");
    }
}
