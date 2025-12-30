use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    sync::mpsc,
    thread,
    time::Duration,
};

use crate::core::{
    ports::Ports,
    target::Target,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortStatus {
    Open,
    Closed,
    Filtered,
}

#[derive(Debug)]
pub struct ScanResult {
    pub port: u16,
    pub status: PortStatus,
    pub service: &'static str, // HAR DOIM BOR
}

const WORKERS: usize = 64;
const TIMEOUT_MS: u64 = 700;

// =======================
// ENTRY
// =======================
pub fn scan(target: &Target, ports: &Ports) -> Vec<ScanResult> {
    let host = target.host.clone();
    let (tx, rx) = mpsc::channel::<ScanResult>();
    let mut handles = Vec::new();

    for batch in ports.ports.chunks(WORKERS) {
        let host = host.clone();
        let tx = tx.clone();
        let list = batch.to_vec();

        let h = thread::spawn(move || {
            for port in list {
                let (status, service) = scan_single(&host, port);
                let _ = tx.send(ScanResult { port, status, service });
            }
        });

        handles.push(h);
    }

    drop(tx);

    let mut results = Vec::new();
    for r in rx {
        results.push(r);
    }

    for h in handles {
        let _ = h.join();
    }

    results.sort_by_key(|r| r.port);
    results
}

// =======================
// CORE LOGIC
// =======================
fn scan_single(host: &str, port: u16) -> (PortStatus, &'static str) {
    let default_service = service_name(port);

    let addrs = match (host, port).to_socket_addrs() {
        Ok(a) => a.collect::<Vec<_>>(),
        Err(_) => return (PortStatus::Filtered, default_service),
    };

    let mut saw_timeout = false;

    for addr in addrs {
        match tcp_connect(addr) {
            TcpResult::Open => {
                // TCP ochiq — endi protocol probe
                if let Some(proto) = protocol_probe(addr, host, port) {
                    return (PortStatus::Open, proto);
                }
                return (PortStatus::Open, default_service);
            }

            TcpResult::Timeout => {
                saw_timeout = true;
            }

            TcpResult::Refused => {}
        }
    }

    if saw_timeout {
        (PortStatus::Filtered, default_service)
    } else {
        (PortStatus::Closed, default_service)
    }
}

// =======================
// TCP CONNECT
// =======================
enum TcpResult {
    Open,
    Refused,
    Timeout,
}

fn tcp_connect(addr: SocketAddr) -> TcpResult {
    match TcpStream::connect_timeout(&addr, Duration::from_millis(TIMEOUT_MS)) {
        Ok(_) => TcpResult::Open,
        Err(e) => {
            use std::io::ErrorKind::*;
            match e.kind() {
                TimedOut | WouldBlock => TcpResult::Timeout,
                ConnectionRefused => TcpResult::Refused,
                _ => TcpResult::Refused,
            }
        }
    }
}

// =======================
// PROTOCOL PROBES
// =======================
fn protocol_probe(addr: SocketAddr, host: &str, port: u16) -> Option<&'static str> {
    match port {
        80 | 8080 | 8000 => http_probe(addr).then_some("HTTP"),
        443 | 8443 => tls_probe(addr, host).then_some("HTTPS"),
        22 => ssh_probe(addr).then_some("SSH"),
        25 => smtp_probe(addr).then_some("SMTP"),
        3306 => mysql_probe(addr).then_some("MYSQL"),
        3389 => rdp_probe(addr).then_some("RDP"),
        _ => None,
    }
}

fn http_probe(addr: SocketAddr) -> bool {
    if let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_millis(TIMEOUT_MS)) {
        let _ = s.write_all(b"HEAD / HTTP/1.1\r\nHost: x\r\n\r\n");
        s.set_read_timeout(Some(Duration::from_millis(TIMEOUT_MS))).ok();
        let mut buf = [0u8; 4];
        return s.read(&mut buf).is_ok();
    }
    false
}

fn tls_probe(addr: SocketAddr, _host: &str) -> bool {
    if let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_millis(TIMEOUT_MS)) {
        s.set_read_timeout(Some(Duration::from_millis(TIMEOUT_MS))).ok();
        let _ = s.write_all(&tls_client_hello());
        let mut buf = [0u8; 1];
        return s.read(&mut buf).is_ok();
    }
    false
}

fn ssh_probe(addr: SocketAddr) -> bool {
    if let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_millis(TIMEOUT_MS)) {
        s.set_read_timeout(Some(Duration::from_millis(TIMEOUT_MS))).ok();
        let mut buf = [0u8; 4];
        if s.read(&mut buf).is_ok() {
            return &buf == b"SSH-";
        }
    }
    false
}

fn smtp_probe(addr: SocketAddr) -> bool {
    if let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_millis(TIMEOUT_MS)) {
        s.set_read_timeout(Some(Duration::from_millis(TIMEOUT_MS))).ok();
        let mut buf = [0u8; 3];
        return s.read(&mut buf).is_ok(); // "220"
    }
    false
}

fn mysql_probe(addr: SocketAddr) -> bool {
    if let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_millis(TIMEOUT_MS)) {
        s.set_read_timeout(Some(Duration::from_millis(TIMEOUT_MS))).ok();
        let mut buf = [0u8; 1];
        return s.read(&mut buf).is_ok();
    }
    false
}

fn rdp_probe(addr: SocketAddr) -> bool {
    if let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_millis(TIMEOUT_MS)) {
        s.set_read_timeout(Some(Duration::from_millis(TIMEOUT_MS))).ok();
        let mut buf = [0u8; 1];
        return s.read(&mut buf).is_ok();
    }
    false
}

// =======================
// SERVICE DB (FALLBACK)
// =======================
fn service_name(port: u16) -> &'static str {
    match port {
        1..=19 => "system",
        20 | 21 => "FTP",
        22 => "SSH",
        23 => "TELNET",
        25 => "SMTP",
        53 => "DNS",
        80 => "HTTP",
        110 => "POP3",
        143 => "IMAP",
        443 => "HTTPS",
        445 => "SMB",
        3306 => "MYSQL",
        3389 => "RDP",
        5432 => "POSTGRES",
        6379 => "REDIS",
        8080 => "HTTP-ALT",
        _ => "unknown",
    }
}

// =======================
// TLS CLIENTHELLO (MINIMAL)
// =======================
fn tls_client_hello() -> Vec<u8> {
    vec![
        0x16, 0x03, 0x01, 0x00, 0x2e,
        0x01, 0x00, 0x00, 0x2a,
        0x03, 0x03,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x02, 0x13, 0x01,
        0x01, 0x00,
    ]
}

