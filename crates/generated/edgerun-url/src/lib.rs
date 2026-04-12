//! URL types — from WHATWG URL Living Standard.
//! DO NOT EDIT. Regenerate with: scripts/generate_web_platform.py

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlParserState {
    SchemeStart,
    Scheme,
    NoScheme,
    SpecialRelativeOrAuthority,
    PathOrAuthority,
    Relative,
    RelativeSlash,
    SpecialAuthoritySlashes,
    SpecialAuthorityIgnoreSlashes,
    Query,
    Host,
    Hostname,
    Port,
    PathStart,
    Path,
    CannotBeABaseUrlPath,
    File,
    FileHost,
    FileSlash,
    Fragment,
}

#[derive(Debug, Clone)]
pub struct Url {
    pub href: String,
    pub origin: String,
    pub protocol: String,
    pub username: String,
    pub password: String,
    pub host: String,
    pub hostname: String,
    pub port: Option<String>,
    pub pathname: String,
    pub search: String,
    pub hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialScheme {
    Ftp,
    File,
    Http,
    Https,
    Ws,
    Wss,
}

impl SpecialScheme {
    pub fn default_port(&self) -> Option<u16> {
        match self {
            SpecialScheme::Ftp => Some(21),
            SpecialScheme::File => None,
            SpecialScheme::Http => Some(80),
            SpecialScheme::Https => Some(443),
            SpecialScheme::Ws => Some(80),
            SpecialScheme::Wss => Some(443),
        }
    }
}

/// URLSearchParams operations.
pub trait URLSearchParamsOps {
    fn append(&mut self, name: &str, value: &str);
    fn delete(&mut self, name: &str);
    fn delete_with_value(&mut self, name: &str, value: &str);
    fn get(&self, name: &str) -> Option<String>;
    fn get_all(&self, name: &str) -> Vec<String>;
    fn has(&self, name: &str) -> bool;
    fn has_with_value(&self, name: &str, value: &str) -> bool;
    fn set(&mut self, name: &str, value: &str);
    fn sort(&mut self);
    fn to_string(&self) -> String;
}
