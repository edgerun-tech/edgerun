use edgerun_bare_rt::{UdpAddr, UdpSocket};

#[test]
fn udp_socket_new() {
    let socket = UdpSocket::new();
    let _ = socket;
}

#[test]
fn udp_addr() {
    let addr = UdpAddr::new(0, 8080);
    let _ = addr;
}
