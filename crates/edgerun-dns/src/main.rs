use edgerun_dns::zone::DnsZone;
use edgerun_dns::{DnsServer, DnsServerConfig};
use edgerun_rt::Runtime;

fn main() {
    let rt = Runtime::new_multi_thread().build().expect("failed to build runtime");
    rt.block_on(async {
        if let Err(e) = run_server().await {
            eprintln!("DNS server error: {}", e);
        }
    });
}

async fn run_server() -> Result<(), edgerun_dns::std::io::Error> {
    let config = DnsServerConfig {
        bind_addr: "0.0.0.0:53".to_string(),
        bind_addr_ipv6: None,
        default_ttl: 3600,
        rate_limit_qps: 100,
    };

    let mut server = DnsServer::new(config)?;

    // Create edgerun.tech zone with records from deploy config
    let mut zone = DnsZone::new("edgerun.tech");
    zone.set_default_ttl(3600);
    
    // NS records
    zone.add_ns("ns1.edgerun.tech.");
    zone.add_ns("ns2.edgerun.tech.");
    
    // A records
    zone.add_a("@", edgerun_dns::std::net::Ipv4Addr::new(172, 245, 67, 49), 3600);
    zone.add_a("ns1", edgerun_dns::std::net::Ipv4Addr::new(172, 245, 67, 49), 3600);
    zone.add_a("ns2", edgerun_dns::std::net::Ipv4Addr::new(172, 245, 67, 49), 3600);
    zone.add_a("mail", edgerun_dns::std::net::Ipv4Addr::new(172, 245, 67, 49), 3600);
    zone.add_a("mta-sts", edgerun_dns::std::net::Ipv4Addr::new(172, 245, 67, 49), 3600);
    zone.add_a("blog", edgerun_dns::std::net::Ipv4Addr::new(172, 245, 67, 49), 3600);
    zone.add_a("git", edgerun_dns::std::net::Ipv4Addr::new(172, 245, 67, 49), 3600);
    
    // MX
    zone.add_mx("@", 0, "mail.edgerun.tech.", 3600);
    
    // TXT records
    zone.add_txt("@", "v=spf1 mx -all", 3600);
    zone.add_txt("_dmarc", "v=DMARC1; p=quarantine; rua=mailto:dmarc-reports@edgerun.tech", 3600);
    zone.add_txt("_mta-sts", "v=STSv1; id=2026043001", 3600);
    zone.add_txt("_smtp._tls", "v=TLSRPTv1; rua=mailto:tls-reports@edgerun.tech", 3600);
    zone.add_txt("default._bimi", "v=BIMI1; l=https://mail.edgerun.tech/bimi/logo.svg", 3600);
    zone.add_txt("mail._domainkey", "v=DKIM1; k=rsa; p=MIIBCgKCAQEAy+Enfug7AYUY+u5InnNMGM39BJsmkqkZFpx5HAUe7ffAPBJhVDIJI4OfcewD+04at3W8aXBx/ZYXxPVO2twqj5nIuKpJVAAeKCaJUuMCyciUtZ89bG61zHFtimMekY4YdSRUwN05Ukq1QfSKFz5FF0E7q7/+KlKATLb3lTtzMJK4olNIDj7EnzW2b9W9PIyyzsnjp/YRSL/u6YlZRWkAT62I8AqnpbegdXMt89aWJMkXF7cfR2TJCJb73qU/ACFYSd6asGWfsCFsDh3dtp8lwpJFwmDY2kUlv6HdN0Hp9NuiX+Ck7z+rrsH9YhVlKlQTtna++bjg2XVgS1rE0o0ogQIDAQAB", 86400);
    
    // CAA records
    zone.add_caa("@", false, "issue", "letsencrypt.org", 3600);
    zone.add_caa("@", false, "iodef", "mailto:admin@edgerun.tech", 3600);
    
    // SOA (needed for zone to be authoritative)
    zone.add_soa("ns1.edgerun.tech.", "admin.edgerun.tech.");
    
    server.add_zone(zone).await;

    edgerun_log::info!("DNS server listening on 0.0.0.0:53");
    server.run().await;

    Ok(())
}