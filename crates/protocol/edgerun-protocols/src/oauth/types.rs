//! OAuth 2.0 vocabulary types.

use crate::prelude::*;

/// An OAuth 2.0 / OIDC scope string.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Scope(pub String);

impl Scope {
    pub fn openid() -> Self {
        Scope("openid".into())
    }

    pub fn email() -> Self {
        Scope("email".into())
    }

    pub fn profile() -> Self {
        Scope("profile".into())
    }

    pub fn offline_access() -> Self {
        Scope("offline_access".into())
    }

    /// Parse a space-separated scope string into a list of scopes.
    pub fn parse_list(s: &str) -> Vec<Self> {
        s.split_whitespace().map(|s| Scope(s.to_string())).collect()
    }

    /// Format scopes as a space-separated string for URL/form parameters.
    pub fn format_list(scopes: &[Self]) -> String {
        scopes
            .iter()
            .map(|s| s.0.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// OAuth 2.0 grant type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantType {
    /// Device Authorization Grant (RFC 8628).
    DeviceCode,
    /// Authorization Code Flow with PKCE.
    AuthorizationCode { code: String, redirect_uri: String },
    /// Refresh Token.
    RefreshToken { refresh_token: String },
    /// Client Credentials.
    ClientCredentials,
}

impl GrantType {
    pub fn as_str(&self) -> &'static str {
        match self {
            GrantType::DeviceCode => "urn:ietf:params:oauth:grant-type:device_code",
            GrantType::AuthorizationCode { .. } => "authorization_code",
            GrantType::RefreshToken { .. } => "refresh_token",
            GrantType::ClientCredentials => "client_credentials",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_scope_parse_list() {
        let scopes = Scope::parse_list("openid profile email offline_access");
        assert_eq!(scopes.len(), 4);
        assert_eq!(scopes[0].0, "openid");
        assert_eq!(scopes[3].0, "offline_access");
    }

    #[test]
    fn test_scope_format_list() {
        let scopes = vec![Scope::openid(), Scope::profile()];
        assert_eq!(Scope::format_list(&scopes), "openid profile");
    }

    #[test]
    fn test_scope_presets() {
        assert_eq!(Scope::openid().0, "openid");
        assert_eq!(Scope::email().0, "email");
        assert_eq!(Scope::profile().0, "profile");
        assert_eq!(Scope::offline_access().0, "offline_access");
    }

    #[test]
    fn grant_type_strings_match_specs() {
        assert_eq!(
            GrantType::DeviceCode.as_str(),
            "urn:ietf:params:oauth:grant-type:device_code"
        );
        assert_eq!(
            GrantType::AuthorizationCode {
                code: "code".into(),
                redirect_uri: "https://example.test/cb".into(),
            }
            .as_str(),
            "authorization_code"
        );
        assert_eq!(
            GrantType::RefreshToken {
                refresh_token: "refresh".into(),
            }
            .as_str(),
            "refresh_token"
        );
        assert_eq!(GrantType::ClientCredentials.as_str(), "client_credentials");
    }
}
