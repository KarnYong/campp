/// Port detection and allocation

use std::net::{TcpListener, TcpStream};
use std::time::Duration;

/// Find an available port starting from the preferred port
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

/// Why a port can or can't be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortStatus {
    Free,
    /// Another program listens on it (or holds it).
    InUse,
    /// Windows refuses it: it's inside a range Hyper-V/WSL2/Docker reserved
    /// (`netsh interface ipv4 show excludedportrange protocol=tcp`). Nothing uses
    /// it, so "in use" would send the user hunting for a program that isn't there.
    Reserved { range: Option<(u16, u16)> },
}

/// Classify a port:
/// 1. connect succeeds → something is listening → InUse
/// 2. bind succeeds → Free
/// 3. bind refused with WSAEACCES (10013) on Windows → Reserved, else InUse
pub fn port_status(port: u16) -> PortStatus {
    let addr = format!("127.0.0.1:{}", port);
    if TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_millis(100)).is_ok() {
        return PortStatus::InUse;
    }
    match TcpListener::bind(&addr) {
        Ok(_) => PortStatus::Free,
        Err(e) if is_reserved_os_error(e.raw_os_error(), cfg!(windows)) => {
            PortStatus::Reserved { range: range_containing(port, &excluded_ranges()) }
        }
        Err(_) => PortStatus::InUse,
    }
}

/// Check if a port is available (not in use or reserved)
pub fn is_port_available(port: u16) -> bool {
    port_status(port) == PortStatus::Free
}

/// WSAEACCES (10013) on bind = the port is in an excluded range. WSAEADDRINUSE
/// (10048) is a normal "in use". Reserved ranges only exist on Windows.
fn is_reserved_os_error(raw: Option<i32>, windows: bool) -> bool {
    windows && raw == Some(10013)
}

/// Windows' excluded TCP port ranges. Reading them needs no admin rights. Empty on
/// other OSes or if netsh fails.
fn excluded_ranges() -> Vec<(u16, u16)> {
    #[cfg(target_os = "windows")]
    {
        let cmd = crate::process::manager::configure_no_window(std::process::Command::new("netsh"));
        if let Ok(out) = { cmd }
            .args(["interface", "ipv4", "show", "excludedportrange", "protocol=tcp"])
            .output()
        {
            return parse_excluded_ranges(&String::from_utf8_lossy(&out.stdout));
        }
    }
    Vec::new()
}

/// Rows of two numbers (start, end), optionally followed by `*` (administered).
fn parse_excluded_ranges(text: &str) -> Vec<(u16, u16)> {
    text.lines()
        .filter_map(|line| {
            let mut cols = line.split_whitespace();
            let start = cols.next()?.parse().ok()?;
            let end = cols.next()?.parse().ok()?;
            match cols.next() {
                None | Some("*") => Some((start, end)),
                Some(_) => None,
            }
        })
        .collect()
}

fn range_containing(port: u16, ranges: &[(u16, u16)]) -> Option<(u16, u16)> {
    ranges.iter().copied().find(|&(start, end)| (start..=end).contains(&port))
}

/// First port after `port` (up to +1000) that is free, not reserved, and not one of
/// `avoid` (the other CAMPP services' ports).
pub fn suggest_port(port: u16, avoid: &[u16]) -> Option<u16> {
    suggest_port_with(port, avoid, &excluded_ranges(), is_port_available)
}

fn suggest_port_with(port: u16, avoid: &[u16], ranges: &[(u16, u16)], is_free: impl Fn(u16) -> bool) -> Option<u16> {
    let last = port.saturating_add(1000);
    (port.checked_add(1)?..=last)
        .filter(|p| !avoid.contains(p) && range_containing(*p, ranges).is_none())
        .find(|&p| is_free(p))
}

/// Check if a port is in use (something is listening on it)
pub fn is_port_in_use(port: u16) -> bool {
    let addr = format!("127.0.0.1:{}", port);

    TcpStream::connect_timeout(
        &addr.parse().unwrap(),
        Duration::from_millis(100),
    ).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_port_available_for_unused_port() {
        // Port 59999 is unlikely to be in use
        // This test just verifies the function doesn't panic
        let _result = is_port_available(59999);
    }

    #[test]
    fn test_find_available_port_returns_preferred_if_available() {
        // Find a port that's definitely available
        let test_port = 59998;
        if is_port_available(test_port) {
            let found = find_available_port(test_port);
            assert_eq!(found, test_port);
        }
    }

    // Spec 006 US3: ports reserved by Windows (Hyper-V/WSL/Docker).

    /// Real `netsh interface ipv4 show excludedportrange protocol=tcp` output (CRLF,
    /// with an administered `*` row), from the maintainer's machine 2026-11-24.
    const NETSH: &str = "\r\nProtocol tcp Port Exclusion Ranges\r\n\r\nStart Port    End Port      \r\n----------    --------      \r\n      7949        8048      \r\n      8091        8190      \r\n     22594       22693      \r\n     50000       50059     *\r\n\r\n* - Administered port exclusions.\r\n\r\n";

    #[test]
    fn parses_netsh_excluded_ranges() {
        assert_eq!(
            parse_excluded_ranges(NETSH),
            vec![(7949, 8048), (8091, 8190), (22594, 22693), (50000, 50059)]
        );
        assert!(parse_excluded_ranges("").is_empty());
        assert!(parse_excluded_ranges("garbage\nnot 12 numbers here\n").is_empty());
    }

    #[test]
    fn finds_the_range_holding_a_port() {
        let ranges = parse_excluded_ranges(NETSH);
        assert_eq!(range_containing(8000, &ranges), Some((7949, 8048)));
        assert_eq!(range_containing(7949, &ranges), Some((7949, 8048)));
        assert_eq!(range_containing(8080, &ranges), None);
    }

    #[test]
    fn suggests_a_free_port_outside_ranges_and_other_campp_ports() {
        let ranges = [(7949, 8048)];
        // 8049 is another CAMPP service's port, so skip it.
        assert_eq!(suggest_port_with(8000, &[8049], &ranges, |_| true), Some(8050));
        // Busy ports are skipped too.
        assert_eq!(suggest_port_with(9000, &[], &[], |p| p > 9002), Some(9003));
        // Nothing free within +1000.
        assert_eq!(suggest_port_with(9000, &[], &[], |_| false), None);
        // Never past u16::MAX.
        assert_eq!(suggest_port_with(65535, &[], &[], |_| true), None);
    }

    #[test]
    fn only_windows_access_denied_means_reserved() {
        assert!(is_reserved_os_error(Some(10013), true));  // WSAEACCES
        assert!(!is_reserved_os_error(Some(10048), true)); // WSAEADDRINUSE
        assert!(!is_reserved_os_error(Some(10013), false)); // never on macOS/Linux
        assert!(!is_reserved_os_error(None, true));
    }

    #[test]
    fn a_listening_port_is_in_use() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        assert_eq!(port_status(port), PortStatus::InUse);
        assert!(!is_port_available(port));
    }
}
