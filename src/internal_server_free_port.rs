// src/port.rs
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
use std::sync::OnceLock;

static FREE_PORT: OnceLock<u16> = OnceLock::new();

pub fn init_global_port(start: u16, end: u16) -> u16 {
    *FREE_PORT.get_or_init(|| {
        for port in start..=end {
            let socket = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
            if TcpListener::bind(socket).is_ok() {
                return port;
            }
        }
        panic!("Failed to find free port in range {}-{}", start, end);
    })
}

pub fn get_global_port() -> &'static u16 {
    FREE_PORT.get().expect("Port not initialized")
}
