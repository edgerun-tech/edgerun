use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;

#[cfg(feature = "dkim")]
use edgerun_protocols::email_auth::DnsQuery;

pub type DnsResult<T> = Result<T, String>;

/// DNS capability supplied by the node runtime.
pub trait MailDnsResolver: Send + Sync {
    fn query_txt<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = DnsResult<Vec<String>>> + Send + 'a>>;

    fn query_mx<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = DnsResult<Vec<(u16, String)>>> + Send + 'a>>;
}

/// Adapter from the mail DNS capability to email-auth DNS lookups.
pub(crate) struct MailDnsQuery {
    resolver: Arc<dyn MailDnsResolver>,
}

impl MailDnsQuery {
    pub(crate) fn new(resolver: Arc<dyn MailDnsResolver>) -> Self {
        Self { resolver }
    }
}

#[cfg(feature = "dkim")]
impl DnsQuery for MailDnsQuery {
    async fn query_txt(
        &mut self,
        name: &str,
    ) -> edgerun_protocols::email_auth::std::io::Result<Vec<alloc::string::String>> {
        self.resolver.query_txt(name).await.map_err(|error| {
            edgerun_protocols::email_auth::std::io::Error::new(
                edgerun_protocols::email_auth::std::io::ErrorKind::Other,
                error,
            )
        })
    }
}
