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
    next_id: u32,
}

impl Registry {
    pub fn new() -> Self {
        let mut r = Self {
            resources: HashMap::new(),
            next_id: 2, // 1 is wl_display
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

    /// Allocate a new object id.
    pub fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
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

    /// Get all globals (for wl_registry).
    pub fn globals(&self) -> Vec<(String, u32)> {
        let mut seen: HashMap<String, u32> = HashMap::new();
        for r in self.resources.values() {
            if !r.alive {
                continue;
            }
            seen.entry(r.interface.clone())
                .and_modify(|v| { *v = (*v).max(r.version) })
                .or_insert(r.version);
        }
        seen.into_iter().collect()
    }

    /// Get the next id counter (for allocating ranges).
    pub fn next_id(&self) -> u32 {
        self.next_id
    }
}
