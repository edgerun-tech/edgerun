use edgerun_rt::TcpSocket;

#[test]
fn tcp_socket_new() {
    let socket = TcpSocket::new();
    let _ = socket;
}

#[test]
fn socket_addr() {
    use edgerun_rt::SocketAddr;
    let addr = SocketAddr::new(0, 8080);
    let _ = addr;
}
