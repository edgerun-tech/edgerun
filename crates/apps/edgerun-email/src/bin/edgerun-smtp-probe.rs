//! Minimal SMTP STARTTLS probe using the edgerun email/TLS stack.

use edgerun_email::smtp::client::SmtpClient;

fn main() -> Result<(), String> {
    let addr = std::env::args()
        .nth(1)
        .ok_or_else(|| "usage: edgerun-smtp-probe <host:port>".to_string())?;
    let rt = edgerun_rt::Builder::new_multi_thread()
        .build()
        .map_err(|e| format!("failed to build runtime: {}", e))?;

    rt.block_on(async move {
        let client = SmtpClient::connect(&addr)
            .await
            .map_err(|e| format!("SMTP probe failed for {addr}: {e}"))?;
        if !client.is_tls_active() {
            return Err(format!("SMTP probe did not negotiate STARTTLS for {addr}"));
        }
        println!("SMTP STARTTLS ok: {addr}");
        Ok(())
    })
}
