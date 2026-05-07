//! DNS root server hints for iterative resolution.

use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::net::Ipv4Addr;

/// A single root hint: nameserver name and IPv4 addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootHint {
    /// Nameserver name, for example `a.root-servers.net`.
    pub name: String,
    /// IPv4 addresses of this nameserver.
    pub addrs: Vec<Ipv4Addr>,
}

/// Default root hints: the 13 root server families.
pub fn default_root_hints() -> Vec<RootHint> {
    vec![
        RootHint {
            name: "a.root-servers.net".to_string(),
            addrs: vec![Ipv4Addr::new(198, 41, 0, 4)],
        },
        RootHint {
            name: "b.root-servers.net".to_string(),
            addrs: vec![Ipv4Addr::new(199, 9, 14, 201)],
        },
        RootHint {
            name: "c.root-servers.net".to_string(),
            addrs: vec![Ipv4Addr::new(192, 33, 4, 12)],
        },
        RootHint {
            name: "d.root-servers.net".to_string(),
            addrs: vec![Ipv4Addr::new(199, 7, 91, 13)],
        },
        RootHint {
            name: "e.root-servers.net".to_string(),
            addrs: vec![Ipv4Addr::new(192, 203, 230, 10)],
        },
        RootHint {
            name: "f.root-servers.net".to_string(),
            addrs: vec![Ipv4Addr::new(192, 5, 5, 241)],
        },
        RootHint {
            name: "g.root-servers.net".to_string(),
            addrs: vec![Ipv4Addr::new(192, 112, 36, 4)],
        },
        RootHint {
            name: "h.root-servers.net".to_string(),
            addrs: vec![Ipv4Addr::new(198, 97, 190, 53)],
        },
        RootHint {
            name: "i.root-servers.net".to_string(),
            addrs: vec![Ipv4Addr::new(192, 36, 148, 17)],
        },
        RootHint {
            name: "j.root-servers.net".to_string(),
            addrs: vec![Ipv4Addr::new(192, 58, 128, 30)],
        },
        RootHint {
            name: "k.root-servers.net".to_string(),
            addrs: vec![Ipv4Addr::new(193, 0, 14, 129)],
        },
        RootHint {
            name: "l.root-servers.net".to_string(),
            addrs: vec![Ipv4Addr::new(199, 7, 83, 42)],
        },
        RootHint {
            name: "m.root-servers.net".to_string(),
            addrs: vec![Ipv4Addr::new(202, 12, 27, 33)],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_root_hints_include_all_root_server_families() {
        let hints = default_root_hints();
        assert_eq!(hints.len(), 13);
        assert_eq!(hints[0].name, "a.root-servers.net");
        assert_eq!(hints[12].addrs, vec![Ipv4Addr::new(202, 12, 27, 33)]);
    }
}
