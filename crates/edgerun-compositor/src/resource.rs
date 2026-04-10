//! Resource/object registry — maps object ids to interfaces and handlers.

use std::collections::HashMap;

/// A registered Wayland object.
pub struct Resource {
    /// Object id.
    pub id: u32,
    /// Interface name.
    pub interface: String,
    /// Interface version.
    pub version: u32,
    /// Client that owns this resource.
    pub client_id: u32,
    /// Whether this resource has been destroyed.
    pub alive: bool,
}

/// Resource registry.
pub struct Registry {
    resources: HashMap<u32, Resource>,
}

impl Registry {
    pub fn new() -> Self {
        let mut r = Self {
            resources: HashMap::new(),
        };
        // Pre-register wl_display
        r.resources.insert(1, Resource {
            id: 1,
            interface: "wl_display".to_string(),
            version: 1,
            client_id: 0,
            alive: true,
        });
        r
    }

    /// Register a new resource.
    pub fn register(&mut self, id: u32, interface: &str, version: u32, client_id: u32) {
        self.resources.insert(id, Resource {
            id,
            interface: interface.to_string(),
            version,
            client_id,
            alive: true,
        });
    }

    /// Get a resource.
    pub fn get(&self, id: u32) -> Option<&Resource> {
        self.resources.get(&id).filter(|r| r.alive)
    }

    /// Get interface name for an id.
    pub fn interface(&self, id: u32) -> Option<&str> {
        self.get(id).map(|r| r.interface.as_str())
    }

    /// Destroy a resource.
    pub fn destroy(&mut self, id: u32) {
        if let Some(r) = self.resources.get_mut(&id) {
            r.alive = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_registry_has_wl_display() {
        let reg = Registry::new();
        let display = reg.get(1).unwrap();
        assert_eq!(display.interface, "wl_display");
        assert!(display.alive);
    }

    #[test]
    fn test_register_and_get() {
        let mut reg = Registry::new();
        reg.register(2, "wl_compositor", 4, 1);
        let res = reg.get(2).unwrap();
        assert_eq!(res.interface, "wl_compositor");
        assert_eq!(res.version, 4);
        assert_eq!(res.client_id, 1);
    }

    #[test]
    fn test_destroy_resource() {
        let mut reg = Registry::new();
        reg.register(2, "wl_surface", 4, 1);
        assert!(reg.get(2).is_some());
        reg.destroy(2);
        assert!(reg.get(2).is_none());
    }

    #[test]
    fn test_interface_lookup() {
        let mut reg = Registry::new();
        reg.register(2, "wl_shm", 1, 1);
        assert_eq!(reg.interface(2), Some("wl_shm"));
        assert_eq!(reg.interface(99), None);
    }

    #[test]
    fn test_destroyed_resource_has_no_interface() {
        let mut reg = Registry::new();
        reg.register(2, "wl_seat", 7, 1);
        reg.destroy(2);
        assert_eq!(reg.interface(2), None);
    }
}
