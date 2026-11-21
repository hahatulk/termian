use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_millis(1200);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reply {
    Echo(Option<f64>),
    SshPort(f64),
    NoReply,
    Unresolved,
    Unavailable,
}

pub fn check(address: &str, ssh_port: u16) -> Reply {
    let address = address.trim();
    let address = address
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(address);
    if address.is_empty() || address.starts_with('-') || address.chars().any(char::is_whitespace) {
        return Reply::Unresolved;
    }

    let addresses = match (address, 0).to_socket_addrs() {
        Ok(addresses) => addresses,
        Err(_) => return Reply::Unresolved,
    };
    let mut tried = [false; 2];
    let mut result = Reply::Unresolved;
    for addr in addresses {
        let ip = addr.ip();
        if ip.is_unspecified()
            || ip.is_multicast()
            || matches!(ip, IpAddr::V4(v4) if v4.is_broadcast())
        {
            continue;
        }
        let family = usize::from(addr.is_ipv6());
        if std::mem::replace(&mut tried[family], true) {
            continue;
        }
        match probe(addr, ssh_port, send, |endpoint| {
            let start = Instant::now();
            TcpStream::connect_timeout(&endpoint, TIMEOUT)
                .ok().map(|_| start.elapsed().as_secs_f64() * 1000.)
        }) {
            reply @ (Reply::Echo(_) | Reply::SshPort(_)) => return reply,
            Reply::NoReply => result = Reply::NoReply,
            other if result != Reply::NoReply => result = other,
            _ => {}
        }
    }
    result
}

fn probe(mut addr: SocketAddr, ssh_port: u16, echo: impl FnOnce(SocketAddr) -> Reply, tcp: impl FnOnce(SocketAddr) -> Option<f64>) -> Reply {
    let answer = echo(addr);
    let local = match addr.ip().to_canonical() {
        IpAddr::V4(ip) => ip.is_private() || ip.is_link_local() || ip.is_loopback(),
        IpAddr::V6(ip) => ip.is_unique_local() || ip.is_unicast_link_local() || ip.is_loopback(),
    };
    if local && ssh_port != 0 && !matches!(answer, Reply::Echo(_)) {
        addr.set_port(ssh_port);
        if let Some(ms) = tcp(addr) {
            return Reply::SshPort(ms);
        }
    }
    answer
}

#[cfg(not(windows))]
fn send(addr: SocketAddr) -> Reply {
    use std::io::Read;
    use std::process::{Command, Stdio};

    #[cfg(target_os = "macos")]
    let (program, family) = (
        if addr.is_ipv6() {
            "/sbin/ping6"
        } else {
            "/sbin/ping"
        },
        "-o",
    );
    #[cfg(not(target_os = "macos"))]
    let (program, family) = ("/usr/bin/ping", if addr.is_ipv6() { "-6" } else { "-4" });

    let address = match addr {
        SocketAddr::V4(v4) => v4.ip().to_string(),
        SocketAddr::V6(v6) if v6.scope_id() != 0 => format!("{}%{}", v6.ip(), v6.scope_id()),
        SocketAddr::V6(v6) => v6.ip().to_string(),
    };

    let Ok(mut child) = Command::new(program)
        .args([family, "-n", "-c", "1", &address])
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Reply::Unavailable;
    };

    let deadline = Instant::now() + TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                let mut output = String::new();
                if let Some(stdout) = child.stdout.take() {
                    let _ = stdout.take(4096).read_to_string(&mut output);
                }
                return Reply::Echo(
                    output
                        .split_whitespace()
                        .find_map(|part| part.strip_prefix("time=")?.parse().ok()),
                );
            }
            Ok(Some(_)) => return Reply::NoReply,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(12)),
            wait => {
                let _ = child.kill();
                let _ = child.wait();
                return if wait.is_err() {
                    Reply::Unavailable
                } else {
                    Reply::NoReply
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_port_probe_follows_missing_echo() {
        let local = SocketAddr::from(([127, 0, 0, 1], 0));
        assert_eq!(probe(local, 2222, |_| Reply::NoReply, |addr| {
            assert_eq!(addr, SocketAddr::from(([127, 0, 0, 1], 2222)));
            Some(1.)
        }), Reply::SshPort(1.));
        assert_eq!(probe(local, 2222, |_| Reply::Echo(None), |_| panic!()), Reply::Echo(None));
        let public = SocketAddr::from(([93, 184, 215, 14], 0));
        assert_eq!(probe(public, 2222, |_| Reply::NoReply, |_| panic!()), Reply::NoReply);
    }
}

#[cfg(windows)]
fn send(addr: SocketAddr) -> Reply {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{GetLastError, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        Icmp6CreateFile, Icmp6SendEcho2, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho,
        ICMPV6_ECHO_REPLY_LH, ICMP_ECHO_REPLY, IP_GENERAL_FAILURE, IP_STATUS_BASE, IP_SUCCESS,
    };
    use windows_sys::Win32::Networking::WinSock::{
        AF_INET6, IN6_ADDR, IN6_ADDR_0, SOCKADDR_IN6, SOCKADDR_IN6_0,
    };

    let data = [0u8; 32];
    let mut buffer = [0u64; 64];
    let reply = buffer.as_mut_ptr().cast();
    let length = std::mem::size_of_val(&buffer) as u32;
    let timeout = TIMEOUT.as_millis() as u32;
    unsafe {
        let handle = if addr.is_ipv6() {
            Icmp6CreateFile()
        } else {
            IcmpCreateFile()
        };
        if handle == INVALID_HANDLE_VALUE {
            return Reply::Unavailable;
        }
        let count = match addr {
            SocketAddr::V4(v4) => IcmpSendEcho(
                handle,
                u32::from_ne_bytes(v4.ip().octets()),
                data.as_ptr().cast(),
                data.len() as u16,
                null(),
                reply,
                length,
                timeout,
            ),
            SocketAddr::V6(v6) => {
                let source = SOCKADDR_IN6 {
                    sin6_family: AF_INET6,
                    ..Default::default()
                };
                let destination = SOCKADDR_IN6 {
                    sin6_family: AF_INET6,
                    sin6_addr: IN6_ADDR {
                        u: IN6_ADDR_0 {
                            Byte: v6.ip().octets(),
                        },
                    },
                    Anonymous: SOCKADDR_IN6_0 {
                        sin6_scope_id: v6.scope_id(),
                    },
                    ..Default::default()
                };
                Icmp6SendEcho2(
                    handle,
                    null_mut(),
                    None,
                    null(),
                    &source,
                    &destination,
                    data.as_ptr().cast(),
                    data.len() as u16,
                    null(),
                    reply,
                    length,
                    timeout,
                )
            }
        };
        let error = if count == 0 { GetLastError() } else { 0 };
        IcmpCloseHandle(handle);
        if count == 0 {
            return if (IP_STATUS_BASE..=IP_GENERAL_FAILURE).contains(&error) {
                Reply::NoReply
            } else {
                Reply::Unavailable
            };
        }
        let (status, ms) = if addr.is_ipv6() {
            let echo = &*reply.cast::<ICMPV6_ECHO_REPLY_LH>();
            (echo.Status, echo.RoundTripTime)
        } else {
            let echo = &*reply.cast::<ICMP_ECHO_REPLY>();
            (echo.Status, echo.RoundTripTime)
        };
        if status == IP_SUCCESS {
            Reply::Echo(Some(f64::from(ms)))
        } else {
            Reply::NoReply
        }
    }
}
