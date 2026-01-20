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
    pub service: &'static str,          // HAR DOIM BOR
    pub os_hint: Option<&'static str>,  // OS SIGNAL (ixtiyoriy)
    pub confidence: u8,                 // 0–100 (v0.2.0)
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
                let result = scan_single(&host, port);
                let _ = tx.send(result);
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

fn scan_single(host: &str, port: u16) -> ScanResult {
    let fallback_service = service_name(port);

    let addrs = match (host, port).to_socket_addrs() {
        Ok(a) => a.collect::<Vec<_>>(),
        Err(_) => {
            return ScanResult {
                port,
                status: PortStatus::Filtered,
                service: fallback_service,
                os_hint: None,
                confidence: 0,
            };
        }
    };

    let mut saw_timeout = false;

    for addr in addrs {
        match tcp_connect(addr) {
            TcpResult::Open => {
                // 1️⃣ SERVICE DETECTION (PROBE → FALLBACK)
                let service = protocol_probe(addr, host, port)
                    .unwrap_or(fallback_service);

                // 2️⃣ OS SIGNAL (YENGIL, MAJBURIY EMAS)
                let os_hint = os_detect_signal(port, service);

                // 3️⃣ CONFIDENCE (v0.2.0 YANGILIGI)
                let confidence = confidence_score(service, &os_hint);

                return ScanResult {
                    port,
                    status: PortStatus::Open,
                    service,
                    os_hint,
                    confidence,
                };
            }
            TcpResult::Timeout => saw_timeout = true,
            TcpResult::Refused => {}
        }
    }

    ScanResult {
        port,
        status: if saw_timeout {
            PortStatus::Filtered
        } else {
            PortStatus::Closed
        },
        service: fallback_service,
        os_hint: None,
        confidence: 0,
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
// SERVICE DETECTION (PROBES)
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
        return s.read(&mut buf).is_ok();
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
// OS SIGNAL (YENGIL, XAVFSIZ)
// =======================
fn os_detect_signal(port: u16, service: &str) -> Option<&'static str> {
    match (port, service) {
        (445, "SMB") | (3389, "RDP") => Some("Windows"),
        (22, "SSH") => Some("Unix-like"),
        (80, "HTTP") | (443, "HTTPS") => Some("Unix-like"),
        _ => None,
    }
}

// =======================
// CONFIDENCE SCORE (v0.2.0 YANGI)
// =======================
fn confidence_score(service: &str, os: &Option<&str>) -> u8 {
    match (service, os) {
        ("SSH", Some(_)) => 95,
        ("HTTP", Some(_)) => 90,
        ("HTTPS", Some(_)) => 90,
        ("RDP", Some("Windows")) => 95,
        ("SMTP", _) => 85,
        ("MYSQL", _) => 85,
        ("unknown", _) => 30,
        _ => 60,
    }
}

// =======================
// FALLBACK SERVICE DB
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
// TLS CLIENT HELLO (MINIMAL)
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

