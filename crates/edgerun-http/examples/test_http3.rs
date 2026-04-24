use edgerun_http::{HttpServer, HttpClient, HttpVersion, into_handler, Response, StatusCode};
use edgerun_http::http3::quic::packet::QuicPacket;
use edgerun_rt::{Runtime, spawn, sleep};
use std::time::Duration;
use std::sync::atomic::{AtomicU32, Ordering};

static PORT: AtomicU32 = AtomicU32::new(14000);

fn next_port() -> u16 {
    PORT.fetch_add(1, Ordering::Relaxed) as u16
}

fn main() {
    println!("=== Test AAD mismatch ===");
    
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    
    rt.block_on(async {
        // Test: Compare AAD from packet vs header_to_bytes_aad
        let pkt = QuicPacket::initial(
            0x00000001,  // QUIC_VERSION_V1
            vec![1,2,3,4,5,6,7,8],  // dcid  
            vec![8,7,6,5,4,3,2,1],  // scid
            vec![],  // token
            0,  // packet number
            vec![0x06, 0x00, 0x40, 0xb0, 0x01],  // CRYPTO frame
        );
        
        let packet_bytes = pkt.to_bytes();
        println!("packet_bytes[:30]={:02x?}", &packet_bytes[..packet_bytes.len().min(30)]);
        
        // Method 1: header_to_bytes_aad()
        let aad_from_method = pkt.header_to_bytes_aad();
        println!("header_to_bytes_aad[:30]={:02x?}", &aad_from_method[..aad_from_method.len().min(30)]);
        
        // Method 2: get_long_header_payload_offset  
        use edgerun_http::http3::quic::packet::get_long_header_payload_offset;
        let offset = get_long_header_payload_offset(&packet_bytes).unwrap();
        let aad_from_split = &packet_bytes[..offset];
        println!("get_long_header_payload_offset[:30]={:02x?}", &aad_from_split[..aad_from_split.len().min(30)]);
        
        // Compare
        if aad_from_method == aad_from_split {
            println!("✓ AAD methods MATCH");
        } else {
            println!("✗ AAD methods DIFFER!");
            println!("  method len={}, split len={}", aad_from_method.len(), aad_from_split.len());
        }
        
        println!("=== Test done ===");
    });
}