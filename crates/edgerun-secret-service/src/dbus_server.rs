//! D-Bus Secret Service server — Unix domain socket listener with full
//! org.freedesktop.Secret.Service implementation.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

use crate::backend::Backend;
use crate::dbus_types::*;
use crate::dbus_wire::{encode_msg, decode_msg};
use crate::session::{SessionManager, Algorithm};

// ===========================================================================
// Server
// ===========================================================================

pub struct Server {
    listener: UnixListener,
    backend: Backend,
    sessions: SessionManager,
    aliases: HashMap<String, String>, // alias name -> collection path
    serial: u32,
}

impl Server {
    /// Binds to a Unix socket path and returns a server.
    pub fn bind(socket_path: &Path, data_root: std::path::PathBuf) -> io::Result<Self> {
        if socket_path.exists() {
            let _ = std::fs::remove_file(socket_path);
        }
        if let Some(parent) = socket_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let listener = UnixListener::bind(socket_path)?;
        let backend = Backend::new(data_root)?;

        // Default "default" alias
        let mut aliases = HashMap::new();
        aliases.insert("default".into(), "/org/freedesktop/secrets/collections/default".into());

        eprintln!("edgerun-secret-service: listening on {}", socket_path.display());

        Ok(Self {
            listener,
            backend,
            sessions: SessionManager::new(),
            aliases,
            serial: 0,
        })
    }

    /// Accept one client connection and serve it to completion.
    pub fn accept_once(&mut self) -> io::Result<()> {
        let (stream, _) = self.listener.accept()?;
        self.serve_client(stream)
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
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(_) => break,
            };
            buf.extend_from_slice(&msg_buf[..n]);

            // Try to decode one or more complete messages
            loop {
                match decode_msg(&buf) {
                    Ok(msg) => {
                        let msg_len = estimate_encoded_len(&msg);
                        buf.drain(..msg_len.min(buf.len()));
                        let reply = self.handle_message(&client_name, &msg);
                        let reply_data = encode_msg(&reply);
                        let _ = stream.write_all(&reply_data);
                        let _ = stream.flush();
                    }
                    Err(_) => break, // incomplete message, wait for more data
                }
            }
        }

        Ok(())
    }

    /// Handle a single D-Bus message and produce a reply.
    fn handle_message(&mut self, client: &str, msg: &Msg) -> Msg {
        let ser = { self.serial += 1; self.serial };

        match msg.mt {
            MType::Call => self.handle_call(client, msg, ser),
            _ => Msg::err(ser, client, "org.freedesktop.DBus.Error.UnknownMethod", "unsupported message type"),
        }
    }

    fn handle_call(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let iface = msg.iface().unwrap_or("");
        let member = msg.member().unwrap_or("");

        match (iface, member) {
            // org.freedesktop.Secret.Service
            ("org.freedesktop.Secret.Service", "OpenSession") => self.open_session(client, msg, ser),
            ("org.freedesktop.Secret.Service", "CreateCollection") => self.create_collection(client, msg, ser),
            ("org.freedesktop.Secret.Service", "SearchItems") => self.search_items(client, msg, ser),
            ("org.freedesktop.Secret.Service", "Unlock") => self.unlock(client, msg, ser),
            ("org.freedesktop.Secret.Service", "Lock") => self.lock(client, msg, ser),
            ("org.freedesktop.Secret.Service", "GetSecrets") => self.get_secrets(client, msg, ser),
            ("org.freedesktop.Secret.Service", "ReadAlias") => self.read_alias(client, msg, ser),
            ("org.freedesktop.Secret.Service", "SetAlias") => self.set_alias(client, msg, ser),

            // org.freedesktop.Secret.Collection
            ("org.freedesktop.Secret.Collection", "ListItems") => self.list_items(client, msg, ser),
            ("org.freedesktop.Secret.Collection", "CreateItem") => self.create_item(client, msg, ser),
            ("org.freedesktop.Secret.Collection", "Delete") => self.delete_collection(client, msg, ser),

            // org.freedesktop.Secret.Item
            ("org.freedesktop.Secret.Item", "GetSecret") => self.get_secret(client, msg, ser),
            ("org.freedesktop.Secret.Item", "Delete") => self.delete_item(client, msg, ser),

            // org.freedesktop.Secret.Session
            ("org.freedesktop.Secret.Session", "Close") => self.close_session(client, msg, ser),

            // Introspection
            ("org.freedesktop.DBus.Introspectable", "Introspect") => self.introspect(client, msg, ser),
            ("org.freedesktop.DBus.Properties", "GetAll") => self.properties_get_all(client, msg, ser),

            _ => Msg::err(ser, client, "org.freedesktop.DBus.Error.UnknownMethod",
                &format!("unknown method {}/{}", iface, member)),
        }
    }

    // ===========================================================================
    // org.freedesktop.Secret.Service methods
    // ===========================================================================

    /// OpenSession (IN String algorithm, IN Variant input, OUT Variant output, OUT ObjectPath result)
    fn open_session(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let algorithm = msg.body.get(0).and_then(Val::s).unwrap_or("plain");
        let _input = msg.body.get(1); // Variant input (ignored for plain)

        match algorithm {
            "plain" => {
                let path = self.sessions.create_session(client, Algorithm::Plain);
                Msg::ret(ser, client).body(vec![
                    Val::Var(Box::new(Val::S("".into()))),
                    Val::O(path),
                ], "vo")
            }
            "dh-ietf1024-sha256-aes128-cbc-pkcs7" => {
                // Not implemented yet — client should fall back to plain
                Msg::err(ser, client, "org.freedesktop.DBus.Error.NotSupported",
                    "dh-ietf1024-sha256-aes128-cbc-pkcs7 not yet implemented; use 'plain'")
            }
            _ => {
                Msg::err(ser, client, "org.freedesktop.DBus.Error.NotSupported",
                    &format!("unsupported algorithm: {}", algorithm))
            }
        }
    }

    /// CreateCollection (IN Dict<String,Variant> properties, IN String alias, OUT ObjectPath collection, OUT ObjectPath prompt)
    fn create_collection(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let properties = msg.body.get(0).and_then(Val::dict_ss).unwrap_or_default();
        let _alias = msg.body.get(1).and_then(Val::s).unwrap_or("");

        let label = properties.iter()
            .find(|(k, _)| k == "org.freedesktop.Secret.Collection.Label")
            .map(|(_, v)| v.as_str())
            .unwrap_or("unnamed");

        let coll_path = format!("/org/freedesktop/secrets/collections/{}", label.replace(' ', "_"));

        // Ensure the collection namespace exists (put a dummy entry to create it)
        if !self.backend.collection_exists(&coll_path) {
            // Just verify we can list it — namespace will be created on first put
            let _ = self.backend.list(&coll_path);
        }

        Msg::ret(ser, client).body(vec![
            Val::O(coll_path),
            Val::O("/".into()), // No prompt needed
        ], "oo")
    }

    /// SearchItems (IN Dict<String,String> attributes, OUT Array<ObjectPath> unlocked, OUT Array<ObjectPath> locked)
    fn search_items(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let attrs = msg.body.get(0).and_then(Val::dict_ss).unwrap_or_default();
        let attr_pairs: Vec<(String, String)> = attrs.into_iter().collect();

        let mut unlocked = Vec::new();
        let locked = Vec::new(); // We don't implement locking, everything is "unlocked"

        // Search across all collections
        let collections = self.backend.list_collections().unwrap_or_default();
        for coll in &collections {
            let results = self.backend.search(coll, &attr_pairs).unwrap_or_default();
            for (key, _meta) in results {
                let item_path = Backend::item_path(coll, &key);
                unlocked.push(Val::O(item_path));
            }
        }

        Msg::ret(ser, client).body(vec![
            Val::Arr(unlocked),
            Val::Arr(locked),
        ], "aoao")
    }

    /// Unlock (IN Array<ObjectPath> objects, OUT Array<ObjectPath> unlocked, OUT ObjectPath prompt)
    fn unlock(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        // We don't implement locking — everything is always unlocked
        let objects = msg.body.get(0).and_then(Val::ao).unwrap_or_default();
        let unlocked: Vec<Val> = objects.into_iter().map(Val::O).collect();

        Msg::ret(ser, client).body(vec![
            Val::Arr(unlocked),
            Val::O("/".into()), // No prompt
        ], "ao")
    }

    /// Lock (IN Array<ObjectPath> objects, OUT Array<ObjectPath> locked, OUT ObjectPath prompt)
    fn lock(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        // We don't implement locking
        let _objects = msg.body.get(0).and_then(Val::ao).unwrap_or_default();

        Msg::ret(ser, client).body(vec![
            Val::Arr(vec![]),
            Val::O("/".into()),
        ], "ao")
    }

    /// GetSecrets (IN Array<ObjectPath> items, IN ObjectPath session, OUT Dict<ObjectPath,Secret> secrets)
    fn get_secrets(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let item_paths = msg.body.get(0).and_then(Val::ao).unwrap_or_default();
        let session_path = msg.body.get(1).and_then(Val::o).unwrap_or("");

        // Look up the session
        let Some(session) = self.sessions.get(session_path) else {
            return Msg::err(ser, client, "org.freedesktop.Secret.Error.NoSuchSession",
                "session not found");
        };

        if session.closed {
            return Msg::err(ser, client, "org.freedesktop.Secret.Error.NoSuchSession",
                "session is closed");
        }

        // Resolve each item path to (collection, key)
        let mut secrets_dict = Vec::new();
        for item_path in &item_paths {
            if let Some((coll, key)) = resolve_item_path(item_path) {
                if let Ok(Some((secret_bytes, _meta))) = self.backend.get(&coll, &key) {
                    // Return secret as-is (plain mode)
                    let secret_bytes_val: Vec<Val> = secret_bytes.iter().map(|&b| Val::Y(b)).collect();
                    let content_type = "text/plain; charset=utf8";

                    // Secret struct: (oa{sv}ays)
                    let secret_struct = Val::Str(vec![
                        Val::O(session_path.to_string()),
                        Val::Dict(vec![]),
                        Val::Arr(secret_bytes_val),
                        Val::S(content_type.into()),
                    ]);

                    secrets_dict.push((Val::O(item_path.clone()), Val::Var(Box::new(secret_struct))));
                }
            }
        }

        Msg::ret(ser, client).body(vec![
            Val::Dict(secrets_dict),
        ], "a{o(v)}")
    }

    /// ReadAlias (IN String name, OUT ObjectPath collection)
    fn read_alias(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let name = msg.body.get(0).and_then(Val::s).unwrap_or("");
        let collection = self.aliases.get(name)
            .cloned()
            .unwrap_or_else(|| "/".into());

        Msg::ret(ser, client).body(vec![
            Val::O(collection),
        ], "o")
    }

    /// SetAlias (IN String name, IN ObjectPath collection)
    fn set_alias(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let name = msg.body.get(0).and_then(Val::s).unwrap_or("");
        let collection = msg.body.get(1).and_then(Val::o).unwrap_or("");

        if name.is_empty() || collection.is_empty() {
            return Msg::err(ser, client, "org.freedesktop.DBus.Error.InvalidArgs",
                "name and collection must be non-empty");
        }

        self.aliases.insert(name.to_string(), collection.to_string());

        Msg::ret(ser, client).body(vec![], "")
    }

    // ===========================================================================
    // org.freedesktop.Secret.Collection methods
    // ===========================================================================

    /// ListItems (OUT Array<ObjectPath> items)
    fn list_items(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let coll = msg.path().unwrap_or("");
        let items = self.backend.list(coll).unwrap_or_default();

        let item_paths: Vec<Val> = items.into_iter()
            .map(|(key, _)| Val::O(Backend::item_path(coll, &key)))
            .collect();

        Msg::ret(ser, client).body(vec![
            Val::Arr(item_paths),
        ], "ao")
    }

    /// CreateItem (IN Dict<String,Variant> properties, IN Secret secret, IN Boolean replace,
    ///             OUT ObjectPath item, OUT ObjectPath prompt)
    fn create_item(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let coll = msg.path().unwrap_or("");

        let properties = msg.body.get(0).and_then(Val::dict_ss).unwrap_or_default();
        let label = properties.iter()
            .find(|(k, _)| k == "org.freedesktop.Secret.Item.Label")
            .map(|(_, v)| v.as_str())
            .unwrap_or("unnamed")
            .to_string();

        let attrs: Vec<(String, String)> = properties.iter()
            .filter(|(k, _)| k.starts_with("org.freedesktop.Secret.Item.Attribute."))
            .map(|(k, v)| (k.strip_prefix("org.freedesktop.Secret.Item.Attribute.").unwrap_or(k).to_string(), v.clone()))
            .collect();

        // Parse secret struct: (oa{sv}ays)
        let Some(secret_val) = msg.body.get(1) else {
            return Msg::err(ser, client, "org.freedesktop.DBus.Error.InvalidArgs",
                "secret is required");
        };

        let secret_bytes = if let Val::Str(fields) = secret_val.clone() {
            if fields.len() >= 3 {
                if let Val::Arr(bytes) = &fields[2] {
                    bytes.iter().filter_map(|b| if let Val::Y(v) = b { Some(*v) } else { None }).collect()
                } else { Vec::new() }
            } else { Vec::new() }
        } else { Vec::new() };

        // Generate a stable item key from label + attributes
        let item_key = Backend::item_key(&label, &attrs);

        let replace = msg.body.get(2).and_then(Val::b).unwrap_or(false);

        if replace {
            let _ = self.backend.delete(coll, &item_key);
        }

        if let Err(e) = self.backend.put(coll, &item_key, &secret_bytes, &label, &attrs) {
            return Msg::err(ser, client, "org.freedesktop.Secret.Error.Failed",
                &e.to_string());
        }

        let item_path = Backend::item_path(coll, &item_key);

        Msg::ret(ser, client).body(vec![
            Val::O(item_path),
            Val::O("/".into()),
        ], "oo")
    }

    /// Delete a collection
    fn delete_collection(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let coll = msg.path().unwrap_or("");
        let items = self.backend.list(coll).unwrap_or_default();

        for (key, _) in items {
            let _ = self.backend.delete(coll, &key);
        }

        // Remove alias if present
        self.aliases.retain(|_, v| v == coll);

        Msg::ret(ser, client).body(vec![
            Val::O("/".into()),
        ], "o")
    }

    // ===========================================================================
    // org.freedesktop.Secret.Item methods
    // ===========================================================================

    /// GetSecret (IN ObjectPath session, OUT Secret secret)
    fn get_secret(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let session_path = msg.body.get(0).and_then(Val::o).unwrap_or("");
        let item_path = msg.path().unwrap_or("");

        let Some(session) = self.sessions.get(session_path) else {
            return Msg::err(ser, client, "org.freedesktop.Secret.Error.NoSuchSession",
                "session not found");
        };

        if session.closed {
            return Msg::err(ser, client, "org.freedesktop.Secret.Error.NoSuchSession",
                "session is closed");
        }

        let Some((coll, key)) = resolve_item_path(item_path) else {
            return Msg::err(ser, client, "org.freedesktop.DBus.Error.InvalidArgs",
                "invalid item path");
        };

        let Ok(Some((secret_bytes, _meta))) = self.backend.get(&coll, &key) else {
            return Msg::err(ser, client, "org.freedesktop.Secret.Error.NoSuchItem",
                "item not found");
        };

        // Return as-is (plain mode)
        let secret_bytes_val: Vec<Val> = secret_bytes.iter().map(|&b| Val::Y(b)).collect();

        let secret_struct = Val::Str(vec![
            Val::O(session_path.to_string()),
            Val::Dict(vec![]),
            Val::Arr(secret_bytes_val),
            Val::S("text/plain; charset=utf8".into()),
        ]);

        Msg::ret(ser, client).body(vec![
            Val::Var(Box::new(secret_struct)),
        ], "v")
    }

    /// Delete an item
    fn delete_item(&mut self, client: &str, msg: &Msg, ser: u32) -> Msg {
        let item_path = msg.path().unwrap_or("");

        let Some((coll, key)) = resolve_item_path(item_path) else {
            return Msg::err(ser, client, "org.freedesktop.DBus.Error.InvalidArgs",
                "invalid item path");
        };

        let existed = self.backend.delete(&coll, &key).unwrap_or(false);
        if !existed {
            return Msg::err(ser, client, "org.freedesktop.Secret.Error.NoSuchItem",
                "item not found");
        }

        Msg::ret(ser, client).body(vec![
            Val::O("/".into()),
        ], "o")
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
        let _iface = msg.body.get(0).and_then(Val::s).unwrap_or("");
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

/// Estimate the encoded length of a D-Bus message for buffer management.
fn estimate_encoded_len(msg: &Msg) -> usize {
    // Header is at least 16 bytes + header fields + body
    16 + 256 + msg.body.len() * 32
}

/// Build D-Bus introspection XML.
fn build_introspect_xml(path: &str) -> String {
    let mut xml = String::from("<!DOCTYPE node PUBLIC \"-//freedesktop//DTD D-BUS Object Introspection 1.0//EN\"\n\"http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd\">\n<node>\n");

    if path == "/org/freedesktop/secrets" {
        xml.push_str(r#"<interface name="org.freedesktop.Secret.Service">
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
"#);
    }

    xml.push_str(r#"<interface name="org.freedesktop.DBus.Introspectable">
<method name="Introspect"><arg name="data" type="s" direction="out"/></method>
</interface>
"#);

    // Add collection child nodes
    if path == "/org/freedesktop/secrets" {
        xml.push_str(r#"<node name="collection"/>
<node name="aliases"/>
"#);
    }

    xml.push_str("</node>\n");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_root() -> PathBuf {
        static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = C.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("srv_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn resolve_item_path_valid() {
        let (coll, key) = resolve_item_path("/org/freedesktop/secrets/collections/default/mykey").unwrap();
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
        assert_eq!(Backend::coll_to_ns("/org/freedesktop/secrets/collections/default"), "default");
    }

    #[test]
    fn item_path_format() {
        assert_eq!(
            Backend::item_path("/org/freedesktop/secrets/collections/default", "mykey"),
            "/org/freedesktop/secrets/collections/default/mykey"
        );
    }
}
