//! Port forwarding over an established russh session, for libtermius'
//! `localPortForwarding`, `remotePortForwarding` and `dynamicPortForwarding`
//! (also used for host chains). Local and dynamic listeners default to
//! 127.0.0.1; a rule must name another bind address explicitly.

use crate::ssh_client::{client_handle, Client, ClientHandler, SshClients};
use russh::client::{Handle, Msg};
use russh::Channel;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tokio::io::{copy_bidirectional, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardOptions {
    kind: String,
    #[serde(default)]
    bind_address: String,
    #[serde(default)]
    port: u16,
    #[serde(default)]
    host: String,
    #[serde(default)]
    host_port: u16,
}

async fn pipe(channel: Channel<Msg>, mut socket: TcpStream) {
    let mut stream = channel.into_stream();
    let _ = copy_bidirectional(&mut stream, &mut socket).await;
}

/// Minimal SOCKS5 (no auth, CONNECT only) for dynamic forwarding.
async fn socks5_target(socket: &mut TcpStream) -> std::io::Result<(String, u16)> {
    let invalid = || std::io::Error::new(std::io::ErrorKind::InvalidData, "unsupported SOCKS request");
    let mut header = [0u8; 2];
    socket.read_exact(&mut header).await?;
    if header[0] != 5 {
        return Err(invalid());
    }
    let mut methods = vec![0u8; header[1] as usize];
    socket.read_exact(&mut methods).await?;
    if !methods.contains(&0) {
        socket.write_all(&[5, 0xff]).await?;
        return Err(invalid());
    }
    socket.write_all(&[5, 0]).await?;
    let mut request = [0u8; 4];
    socket.read_exact(&mut request).await?;
    if request[0] != 5 || request[1] != 1 {
        socket.write_all(&[5, 7, 0, 1, 0, 0, 0, 0, 0, 0]).await?;
        return Err(invalid());
    }
    let host = match request[3] {
        1 => {
            let mut address = [0u8; 4];
            socket.read_exact(&mut address).await?;
            std::net::Ipv4Addr::from(address).to_string()
        }
        3 => {
            let mut length = [0u8; 1];
            socket.read_exact(&mut length).await?;
            let mut name = vec![0u8; length[0] as usize];
            socket.read_exact(&mut name).await?;
            String::from_utf8(name).map_err(|_| invalid())?
        }
        4 => {
            let mut address = [0u8; 16];
            socket.read_exact(&mut address).await?;
            std::net::Ipv6Addr::from(address).to_string()
        }
        _ => return Err(invalid()),
    };
    let mut port = [0u8; 2];
    socket.read_exact(&mut port).await?;
    Ok((host, u16::from_be_bytes(port)))
}

async fn serve_local(client: Arc<Client>, handle: Arc<Handle<ClientHandler>>, listener: TcpListener, options: ForwardOptions, dynamic: bool) {
    loop {
        let Ok((mut socket, peer)) = listener.accept().await else {
            break;
        };
        let handle = handle.clone();
        let client = client.clone();
        let (host, port) = (options.host.clone(), options.host_port);
        tauri::async_runtime::spawn(async move {
            let (host, port) = if dynamic {
                match socks5_target(&mut socket).await {
                    Ok(target) => target,
                    Err(_) => return,
                }
            } else {
                (host, port)
            };
            match handle
                .channel_open_direct_tcpip(host.as_str(), port as u32, peer.ip().to_string(), peer.port() as u32)
                .await
            {
                Ok(channel) => {
                    if dynamic && socket.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]).await.is_err() {
                        return;
                    }
                    pipe(channel, socket).await;
                }
                Err(error) => {
                    if dynamic {
                        let _ = socket.write_all(&[5, 5, 0, 1, 0, 0, 0, 0, 0, 0]).await;
                    }
                    client.emit(json!({"kind": "log", "visibility": "user", "severity": "warning",
                        "message": format!("Forwarding to {host}:{port} failed: {error}")}));
                }
            }
        });
    }
}

#[tauri::command]
pub async fn ssh_forward_start(state: tauri::State<'_, SshClients>, id: String, forward: u32, options: ForwardOptions) -> Result<(), String> {
    let (client, handle) = client_handle(&state, &id)?;
    let bind_address = if options.bind_address.trim().is_empty() { "127.0.0.1".to_string() } else { options.bind_address.trim().to_string() };
    match options.kind.as_str() {
        "local" | "dynamic" => {
            let listener = TcpListener::bind((bind_address.as_str(), options.port))
                .await
                .map_err(|error| format!("Cannot listen on {bind_address}:{}: {error}", options.port))?;
            let local = listener.local_addr().map_err(|error| error.to_string())?;
            client.emit(json!({"kind": "forwardReady", "forward": forward, "bindAddress": local.ip().to_string(), "bindPort": local.port()}));
            let dynamic = options.kind == "dynamic";
            let task_client = client.clone();
            client.spawn(async move {
                serve_local(task_client.clone(), handle, listener, options, dynamic).await;
                task_client.emit(json!({"kind": "forwardStopped", "forward": forward}));
            });
        }
        "remote" => {
            let port = handle
                .tcpip_forward(bind_address.as_str(), options.port as u32)
                .await
                .map_err(|error| format!("The server refused remote forwarding on {bind_address}:{}: {error}", options.port))?;
            let port = if options.port == 0 { port } else { options.port as u32 };
            client.add_remote_forward(port, (options.host.clone(), options.host_port));
            client.emit(json!({"kind": "forwardReady", "forward": forward, "bindAddress": bind_address, "bindPort": port}));
        }
        other => return Err(format!("Unsupported type of port forwarding: {other}")),
    }
    Ok(())
}

/// Server-initiated channel for a remote forward: connect to the local target.
pub(crate) async fn accept_remote(client: Arc<Client>, channel: Channel<Msg>, connected_port: u32) {
    let Some((host, port)) = client.remote_forward(connected_port) else {
        return;
    };
    match TcpStream::connect((host.as_str(), port)).await {
        Ok(socket) => pipe(channel, socket).await,
        Err(error) => {
            client.emit(json!({"kind": "log", "visibility": "user", "severity": "warning",
                "message": format!("Remote forward to {host}:{port} failed: {error}")}));
            let _ = channel.close().await;
        }
    }
}
