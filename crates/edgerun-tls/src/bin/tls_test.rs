use std::time::{Duration, Instant};

fn main() {
    println!("=== edgerun TLS connectivity test ===");
    let start = Instant::now();

    let rt = edgerun_rt::Runtime::new_multi_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    let result = rt.block_on(async {
        println!("[{:.3}s] Starting ACME init test...", start.elapsed().as_secs_f64());

        // Test 1: DNS resolution
        println!("[{:.3}s] Test 1: DNS resolution via to_socket_addrs...", start.elapsed().as_secs_f64());
        let dns_start = Instant::now();
        let addrs: Vec<_> = match std::net::ToSocketAddrs::to_socket_addrs("acme-v02.api.letsencrypt.org:443") {
            Ok(a) => a.collect(),
            Err(e) => {
                println!("[{:.3}s] DNS FAILED: {}", start.elapsed().as_secs_f64(), e);
                return Err(format!("DNS: {}", e));
            }
        };
        println!("[{:.3}s] DNS OK ({:?}): {:?}", start.elapsed().as_secs_f64(), dns_start.elapsed(), addrs);

        // Test 2: HTTP client
        println!("[{:.3}s] Test 2: HTTP client GET...", start.elapsed().as_secs_f64());
        let http_start = Instant::now();

        let acme_config = edgerun_acme::AcmeConfig {
            directory_url: edgerun_acme::DirectoryUrl::LetsEncrypt,
            email: vec![],
            terms_of_service_agreed: true,
        };

        let account_key = edgerun_acme::account::AccountKey::generate();
        let client = edgerun_acme::AcmeClient::new(acme_config, account_key);

        match client.init().await {
            Ok(()) => {
                println!("[{:.3}s] ACME init OK ({:?})", start.elapsed().as_secs_f64(), http_start.elapsed());
                Ok(())
            }
            Err(e) => {
                println!("[{:.3}s] ACME init FAILED: {}", start.elapsed().as_secs_f64(), e);
                Err(format!("ACME: {}", e))
            }
        }
    });

    match result {
        Ok(()) => println!("[{:.3}s] ALL TESTS PASSED", start.elapsed().as_secs_f64()),
        Err(e) => println!("[{:.3}s] TEST FAILED: {}", start.elapsed().as_secs_f64(), e),
    }
}
