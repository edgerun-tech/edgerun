//! URI parsing

use std::fmt;
use std::str::FromStr;

/// Parsed URI
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Uri {
    scheme: Scheme,
    authority: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    path: String,
    query: Option<String>,
    fragment: Option<String>,
}

/// URI scheme
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scheme {
    /// HTTP
    Http,
    /// HTTPS
    Https,
    /// Other scheme
    Other(String),
}

impl Uri {
    /// Parse a URI string
    pub fn parse(uri: &str) -> Result<Self, String> {
        if uri.is_empty() {
            return Err("URI cannot be empty".to_string());
        }

        let mut scheme = Scheme::Http;
        let mut remainder = uri;

        // Parse scheme
        if let Some(pos) = uri.find("://") {
            let scheme_str = &uri[..pos];
            scheme = match scheme_str {
                "http" => Scheme::Http,
                "https" => Scheme::Https,
                other => Scheme::Other(other.to_string()),
            };
            remainder = &uri[pos + 3..];
        }

        // Split authority and path+query
        let (authority_part, path_query) = if let Some(pos) = remainder.find('/') {
            (Some(&remainder[..pos]), &remainder[pos..])
        } else {
            // No path, entire remainder is authority
            (Some(remainder), "/")
        };

        // Parse authority (host:port)
        let mut host = None;
        let mut port = None;
        let mut authority = None;

        if let Some(auth) = authority_part {
            authority = Some(auth.to_string());
            // Remove userinfo if present
            let auth_without_userinfo = if let Some(pos) = auth.find('@') {
                &auth[pos + 1..]
            } else {
                auth
            };

            // Parse host:port
            let (host_str, port_str) = if auth_without_userinfo.starts_with('[') {
                // IPv6
                if let Some(bracket_end) = auth_without_userinfo.find(']') {
                    let host = &auth_without_userinfo[..bracket_end + 1];
                    let port = if bracket_end + 1 < auth_without_userinfo.len()
                        && auth_without_userinfo.as_bytes()[bracket_end + 1] == b':'
                    {
                        Some(&auth_without_userinfo[bracket_end + 2..])
                    } else {
                        None
                    };
                    (host, port)
                } else {
                    (auth_without_userinfo, None)
                }
            } else if let Some(pos) = auth_without_userinfo.rfind(':') {
                (&auth_without_userinfo[..pos], Some(&auth_without_userinfo[pos + 1..]))
            } else {
                (auth_without_userinfo, None)
            };

            host = Some(host_str.to_string());
            if let Some(port_str) = port_str {
                port = port_str.parse::<u16>().ok();
            }
        }

        // Apply default ports
        if port.is_none() {
            port = match scheme {
                Scheme::Http => Some(80),
                Scheme::Https => Some(443),
                _ => None,
            };
        }

        // Parse path, query, fragment
        let path_query_fragment = path_query;
        let (path, query, fragment) = if let Some(query_pos) = path_query_fragment.find('?') {
            let (path_part, query_frag) = path_query_fragment.split_at(query_pos);
            let query_frag = &query_frag[1..]; // skip '?'

            let (query_part, fragment_part) = if let Some(frag_pos) = query_frag.find('#') {
                let (q, f) = query_frag.split_at(frag_pos);
                (Some(q.to_string()), Some(f[1..].to_string()))
            } else {
                (Some(query_frag.to_string()), None)
            };

            (
                if path_part.is_empty() {
                    "/".to_string()
                } else {
                    path_part.to_string()
                },
                query_part,
                fragment_part,
            )
        } else if let Some(frag_pos) = path_query_fragment.find('#') {
            let (path_part, fragment_part) = path_query_fragment.split_at(frag_pos);
            (
                if path_part.is_empty() {
                    "/".to_string()
                } else {
                    path_part.to_string()
                },
                None,
                Some(fragment_part[1..].to_string()),
            )
        } else {
            (
                if path_query_fragment.is_empty() {
                    "/".to_string()
                } else {
                    path_query_fragment.to_string()
                },
                None,
                None,
            )
        };

        Ok(Uri {
            scheme,
            authority,
            host,
            port,
            path,
            query,
            fragment,
        })
    }

    /// Get the scheme
    pub fn scheme(&self) -> &Scheme {
        &self.scheme
    }

    /// Get the host
    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    /// Get the port
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// Get the path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the query string
    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    /// Get the fragment
    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_deref()
    }

    /// Check if this is HTTPS
    pub fn is_https(&self) -> bool {
        matches!(self.scheme, Scheme::Https)
    }

    /// Get the request target (path + query)
    pub fn request_target(&self) -> String {
        if let Some(query) = &self.query {
            format!("{}?{}", self.path, query)
        } else {
            self.path.clone()
        }
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.scheme {
            Scheme::Http => write!(f, "http://")?,
            Scheme::Https => write!(f, "https://")?,
            Scheme::Other(s) => write!(f, "{}://", s)?,
        }

        if let Some(authority) = &self.authority {
            write!(f, "{}", authority)?;
        }

        write!(f, "{}", self.path)?;

        if let Some(query) = &self.query {
            write!(f, "?{}", query)?;
        }

        if let Some(fragment) = &self.fragment {
            write!(f, "#{}", fragment)?;
        }

        Ok(())
    }
}

impl FromStr for Uri {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uri::parse(s)
    }
}

impl Scheme {
    /// Check if this scheme is HTTPS
    pub fn is_https(&self) -> bool {
        matches!(self, &Scheme::Https)
    }
}
