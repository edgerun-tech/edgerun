use alloc::string::ToString;
use alloc::vec::Vec;

use edgerun_email_auth::DnsQuery;

/// Adapter between the current email runtime DNS client and email-auth logic.
pub(crate) struct DnsClientQuery<'a>(pub &'a mut edgerun_dns::client::DnsClient);

impl DnsQuery for DnsClientQuery<'_> {
    async fn query_txt(
        &mut self,
        name: &str,
    ) -> edgerun_email_auth::std::io::Result<Vec<alloc::string::String>> {
        self.0.query_txt(name).await.map_err(|error| {
            edgerun_email_auth::std::io::Error::new(
                edgerun_email_auth::std::io::ErrorKind::Other,
                error.to_string(),
            )
        })
    }
}
