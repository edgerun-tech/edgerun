use url::Url;

use crate::types::OrderStatus;

#[derive(Debug, Clone)]
pub struct Order {
    pub inner: crate::types::Order,
}

impl Order {
    pub fn from_acme(order: crate::types::Order) -> Self {
        Self { inner: order }
    }

    pub fn status(&self) -> OrderStatus {
        self.inner.status
    }

    pub fn is_pending(&self) -> bool {
        self.inner.status == OrderStatus::Pending
    }

    pub fn is_ready(&self) -> bool {
        self.inner.status == OrderStatus::Ready
    }

    pub fn is_valid(&self) -> bool {
        self.inner.status == OrderStatus::Valid
    }

    pub fn is_invalid(&self) -> bool {
        self.inner.status == OrderStatus::Invalid
    }

    pub fn is_processing(&self) -> bool {
        self.inner.status == OrderStatus::Processing
    }

    pub fn authorization_urls(&self) -> &[Url] {
        self.inner.authorizations.as_deref().unwrap_or(&[])
    }

    pub fn finalize_url(&self) -> Option<&Url> {
        self.inner.finalize.as_ref()
    }

    pub fn certificate_url(&self) -> Option<&Url> {
        self.inner.certificate.as_ref()
    }

    pub fn expires(&self) -> Option<&str> {
        self.inner.expires.as_deref()
    }

    pub fn domains(&self) -> Vec<String> {
        self.inner.identifiers.as_ref()
            .map(|ids| ids.iter().map(|i| i.value.clone()).collect())
            .unwrap_or_default()
    }
}
