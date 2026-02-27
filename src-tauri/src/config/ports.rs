/// Port detection and allocation
/// TODO: Implement in Phase 4

use std::net::TcpListener;

pub fn find_available_port(preferred: u16) -> u16 {
    if is_port_available(preferred) {
        return preferred;
    }

    // Try alternative ports
    for port in (preferred + 1)..65535 {
        if is_port_available(port) {
            return port;
        }
    }

    preferred // Fallback
}

pub fn is_port_available(port: u16) -> bool {
    TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok()
}
