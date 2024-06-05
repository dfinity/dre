pub mod cli;
pub mod clients;
pub(crate) mod defaults;
pub mod detect_neuron;
pub mod general;
pub mod ic_admin;
pub mod operations;
pub mod ops_subnet_node_replace;
pub mod parsed_cli;
pub mod registry_dump;
pub mod runner;

/// Get a localhost socket address with random, unused port.
pub fn local_unused_port() -> u16 {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let socket = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, Some(socket2::Protocol::TCP)).unwrap();
    socket.bind(&addr.into()).unwrap();
    socket.set_reuse_address(true).unwrap();
    let tcp = std::net::TcpListener::from(socket);
    tcp.local_addr().unwrap().port()
}
