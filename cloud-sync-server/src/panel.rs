//! Local web panel (default `127.0.0.1:8081`). After the user enters the
//! **encryption password**, the server decrypts the stored snapshot in memory
//! and shows its contents and the server state; the decrypted JSON can be
//! downloaded. Nothing derived from the password is stored or logged.
//! Protections: Host allow-list (loopback by default, `SYNC_PANEL_ALLOWED_HOSTS`
//! adds names/IPs or `*` for any; guards against DNS rebinding), JSON-only API with
//! a custom header (no cross-site form posts), strict CSP, and the same
//! failed-attempt lockout as the bearer token.

use crate::{current, snapshot_path, too_many_attempts, AuthLimiter, Shared, MAX_BACKUPS};
use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use axum::{
    body::Bytes,
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::{env, fs, sync::Arc, time::SystemTime};

const PAGE: &str = include_str!("panel.html");
const ENVELOPE_VERSION: u64 = 1;
const COLLECTIONS: [(&str, &str); 12] = [
    ("hosts", "Хосты"),
    ("groups", "Группы"),
    ("identities", "Идентификаторы"),
    ("sshKeys", "SSH-ключи"),
    ("knownHosts", "Known Hosts"),
    ("portForwardingRules", "Проброс портов"),
    ("snippets", "Сниппеты"),
    ("snippetPackages", "Пакеты сниппетов"),
    ("sshConfigs", "SSH-конфигурации"),
    ("telnetConfigs", "Telnet-конфигурации"),
    ("proxies", "Прокси"),
    ("workspaceTemplates", "Шаблоны рабочих областей"),
];

/// Host names (besides loopback) the panel answers to. `None` = any host.
#[derive(Clone, Debug, PartialEq)]
pub struct AllowedHosts(Option<Vec<String>>);

impl AllowedHosts {
    /// Comma-separated `SYNC_PANEL_ALLOWED_HOSTS`, e.g. `192.168.1.5,sync.example.com`; `*` allows any.
    pub fn parse(value: &str) -> Self {
        let names: Vec<String> = value.split(',').map(|name| name.trim().to_ascii_lowercase()).filter(|name| !name.is_empty()).collect();
        if names.iter().any(|name| name == "*") {
            return Self(None);
        }
        Self(Some(names))
    }

    pub fn from_env() -> Self {
        Self::parse(&env::var("SYNC_PANEL_ALLOWED_HOSTS").unwrap_or_default())
    }

    fn allows(&self, host: &str) -> bool {
        let name = host_name(host);
        match &self.0 {
            None => true,
            Some(names) => is_loopback_name(&name) || names.iter().any(|allowed| *allowed == name),
        }
    }
}

pub fn make_panel(state: Shared) -> Router {
    make_panel_with_hosts(state, AllowedHosts::from_env())
}

pub fn make_panel_with_hosts(state: Shared, hosts: AllowedHosts) -> Router {
    let hosts = Arc::new(hosts);
    Router::new()
        .route("/", get(page))
        .route("/api/unlock", post(unlock))
        .route("/api/export", post(export))
        .route("/api/rotate-password", post(rotate_password))
        .layer(middleware::from_fn(move |request: Request, next: Next| {
            let hosts = hosts.clone();
            async move { host_guard(&hosts, request, next).await }
        }))
        .with_state(state)
}

/// Reject any Host outside the allow-list: a malicious site resolving its own
/// domain to the server's address (DNS rebinding) cannot reach the panel.
async fn host_guard(hosts: &AllowedHosts, request: Request, next: Next) -> Response {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if !hosts.allows(host) {
        return (StatusCode::FORBIDDEN, "panel host is not allowed; add it to SYNC_PANEL_ALLOWED_HOSTS\n").into_response();
    }
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; img-src data:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    response
}

fn host_name(host: &str) -> String {
    let name = if let Some(rest) = host.strip_prefix('[') {
        rest.split(']').next().unwrap_or("")
    } else {
        host.rsplit_once(':').map_or(host, |(name, _)| name)
    };
    name.to_ascii_lowercase()
}

fn is_loopback_name(name: &str) -> bool {
    matches!(name, "127.0.0.1" | "localhost" | "::1")
}

#[cfg(test)]
fn is_loopback_host(host: &str) -> bool {
    is_loopback_name(&host_name(host))
}

async fn page() -> Response {
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], PAGE).into_response()
}

#[derive(Deserialize)]
struct Unlock {
    passphrase: String,
}

fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({"error": message}))).into_response()
}

/// JSON body plus `X-Termian-Panel: 1`: browsers cannot send either from a
/// cross-site HTML form without a CORS preflight, which is never granted.
fn read_request<T: serde::de::DeserializeOwned>(headers: &HeaderMap, body: &Bytes) -> Result<T, Response> {
    let json_body = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"));
    if !json_body || headers.get("x-termian-panel").map(|v| v.as_bytes()) != Some(b"1") {
        return Err(error(StatusCode::BAD_REQUEST, "Неверный запрос"));
    }
    serde_json::from_slice(body).map_err(|_| error(StatusCode::BAD_REQUEST, "Неверный запрос"))
}

enum Opened {
    Missing,
    Decrypted { snapshot: Value, bytes: usize, revision: String },
}

/// Decrypt the stored snapshot with the limiter applied. The client's wrong
/// passphrases consume the same kind of lockout as wrong bearer tokens.
fn decrypt_slot(state: &Shared) -> Result<tokio::sync::OwnedSemaphorePermit, Response> {
    state.panel_decrypt.clone().try_acquire_owned()
        .map_err(|_| too_many_attempts(std::time::Duration::from_secs(1)))
}

type Peer = Option<axum::extract::ConnectInfo<std::net::SocketAddr>>;

fn peer_ip(peer: Peer) -> std::net::IpAddr {
    peer.map_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), |peer| peer.0.ip())
}

async fn open_with_limit(state: &Shared, ip: std::net::IpAddr, passphrase: String, export: bool) -> Result<(Opened, Option<Vec<u8>>, tokio::sync::OwnedSemaphorePermit), Response> {
    let permit = decrypt_slot(state)?;
    if let Some(left) = state.panel_limiter.locked(ip) {
        return Err(too_many_attempts(left));
    }
    let storage = state.clone();
    let (result, permit) = tokio::task::spawn_blocking(move || {
        let result = (|| -> Result<(Opened, Option<Vec<u8>>), &'static str> {
            let Some(data) = crate::with_storage(&storage, |cfg| current(&panel_scope(cfg)?.scoped(cfg))).map_err(|_| "storage")? else {
                return Ok((Opened::Missing, None));
            };
            let snapshot = open_envelope(&data, &passphrase)?;
            let body = if export {
                Some(serde_json::to_vec(&snapshot).map_err(|_| "format")?)
            } else {
                None
            };
            Ok((Opened::Decrypted { snapshot, bytes: data.len(), revision: crate::digest(&data) }, body))
        })();
        (result, permit)
    })
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "Внутренняя ошибка"))?;
    match result {
        Ok((opened, body)) => {
            if matches!(opened, Opened::Decrypted { .. }) {
                state.panel_limiter.succeed(ip);
            }
            Ok((opened, body, permit))
        }
        Err("storage") => Err(error(StatusCode::INTERNAL_SERVER_ERROR, "Не удалось прочитать снимок")),
        Err("format") => Err(error(StatusCode::UNPROCESSABLE_ENTITY, "Снимок повреждён или имеет неизвестный формат")),
        Err(_) => Err(match state.panel_limiter.fail(ip) {
            Some(lockout) => too_many_attempts(lockout),
            None => error(StatusCode::UNAUTHORIZED, "Неверный пароль шифрования"),
        }),
    }
}

/// Same envelope as the Termian client (src/rust/native.rs): scrypt N=2^14,
/// r=8, p=1 → AES-256-GCM, fields base64-encoded in JSON.
pub fn open_envelope(data: &[u8], passphrase: &str) -> Result<Value, &'static str> {
    let envelope: Value = serde_json::from_slice(data).map_err(|_| "format")?;
    if envelope.get("version").and_then(Value::as_u64) != Some(ENVELOPE_VERSION) {
        return Err("format");
    }
    let field = |name: &str, size: Option<usize>| -> Result<Vec<u8>, &'static str> {
        let text = envelope.get(name).and_then(Value::as_str).ok_or("format")?;
        let bytes = STANDARD.decode(text).map_err(|_| "format")?;
        if size.is_some_and(|size| bytes.len() != size) {
            return Err("format");
        }
        Ok(bytes)
    };
    let salt = field("salt", Some(16))?;
    let iv = field("iv", Some(12))?;
    let tag = field("tag", Some(16))?;
    let mut ciphertext = field("ciphertext", None)?;
    ciphertext.extend_from_slice(&tag);
    let params = scrypt::Params::new(14, 8, 1, 32).map_err(|_| "format")?;
    let mut key = [0u8; 32];
    scrypt::scrypt(passphrase.as_bytes(), &salt, &params, &mut key).map_err(|_| "format")?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| "format")?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&iv), ciphertext.as_slice())
        .map_err(|_| "passphrase")?;
    let snapshot: Value = serde_json::from_slice(&plaintext).map_err(|_| "format")?;
    if !snapshot.is_object() {
        return Err("format");
    }
    Ok(snapshot)
}

fn unix_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(SystemTime::UNIX_EPOCH).ok().map(|d| d.as_millis() as u64)
}

async fn server_info(state: &Shared) -> Value {
    let state = state.clone();
    tokio::task::spawn_blocking(move || crate::with_storage(&state, |cfg| Ok(server_info_at(&state, cfg))))
        .await.ok().and_then(Result::ok).unwrap_or_else(|| json!({}))
}

fn server_info_at(state: &Shared, cfg: &crate::Config) -> Value {
    let display = panel_scope(cfg).ok().map(|scope| scope.scoped(cfg));
    let cfg = display.as_ref().unwrap_or(cfg);
    let snapshot = fs::metadata(snapshot_path(cfg)).ok();
    let mut revisions: Vec<(u64, u64)> = fs::read_dir(cfg.dir.join("revisions"))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "snapshot"))
                .filter_map(|entry| {
                    let meta = entry.metadata().ok()?;
                    Some((meta.modified().ok().and_then(unix_ms).unwrap_or(0), meta.len()))
                })
                .collect()
        })
        .unwrap_or_default();
    revisions.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    let last_download = state.last_download.lock().ok().and_then(|value| *value).and_then(unix_ms);
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "uptimeSeconds": state.started.elapsed().as_secs(),
        "startedAt": unix_ms(SystemTime::now()).map(|now| now - state.started.elapsed().as_millis() as u64),
        "snapshotExists": snapshot.is_some(),
        "snapshotBytes": snapshot.as_ref().map(|meta| meta.len()),
        "lastUploadAt": snapshot.as_ref().and_then(|meta| meta.modified().ok()).and_then(unix_ms),
        "lastDownloadAt": last_download,
        "revision": null,
        "revisions": revisions.iter().map(|(at, size)| json!({"at": at, "bytes": size})).collect::<Vec<_>>(),
        "maxRevisions": MAX_BACKUPS,
        "maxBodyBytes": cfg.limit,
        "lockedClients": state.limiter.locked_clients(),
        "authFailures": state.limiter.free_failures,
        "authLockoutSeconds": state.limiter.base_lockout.as_secs(),
        "trustForwardedFor": state.limiter.trust_forwarded_for,
    })
}

fn snapshot_summary(snapshot: &Value, bytes: usize) -> Value {
    let counts: Vec<Value> = COLLECTIONS
        .iter()
        .map(|(key, title)| {
            let count = snapshot.get(*key).and_then(Value::as_array).map_or(0, Vec::len);
            json!({"key": key, "title": title, "count": count})
        })
        .collect();
    let settings = snapshot.get("settings").and_then(Value::as_object).map_or(0, Map::len);
    json!({
        "format": snapshot.get("format"),
        "formatVersion": snapshot.get("version"),
        "createdAt": snapshot.get("createdAt"),
        "encryptedBytes": bytes,
        "decryptedBytes": serde_json::to_vec(snapshot).map(|data| data.len()).unwrap_or(0),
        "collections": counts,
        "settingsCount": settings,
    })
}

async fn unlock(State(state): State<Shared>, peer: Peer, headers: HeaderMap, body: Bytes) -> Response {
    let request: Unlock = match read_request(&headers, &body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match open_with_limit(&state, peer_ip(peer), request.passphrase, false).await {
        Ok((Opened::Missing, _, _permit)) => Json(json!({"server": server_info(&state).await, "snapshot": null})).into_response(),
        Ok((Opened::Decrypted { snapshot, bytes, revision }, _, _permit)) => {
            let mut server = server_info(&state).await;
            server["revision"] = revision.into();
            Json(json!({"server": server, "snapshot": snapshot_summary(&snapshot, bytes)})).into_response()
        }
        Err(response) => response,
    }
}

fn require_panel_token(state: &Shared, ip: std::net::IpAddr, headers: &HeaderMap) -> Result<(), Response> {
    if let Some(left) = state.panel_tokens.locked(ip) {
        return Err(too_many_attempts(left));
    }
    if !crate::authorized(headers, state) {
        return Err(match state.panel_tokens.fail(ip) {
            Some(left) => too_many_attempts(left),
            None => error(StatusCode::UNAUTHORIZED, "Неверный токен доступа"),
        });
    }
    state.panel_tokens.succeed(ip);
    Ok(())
}

async fn export(State(state): State<Shared>, peer: Peer, headers: HeaderMap, body: Bytes) -> Response {
    let request: Unlock = match read_request(&headers, &body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let ip = peer_ip(peer);
    if let Err(response) = require_panel_token(&state, ip, &headers) {
        return response;
    }
    match open_with_limit(&state, ip, request.passphrase, true).await {
        Ok((_, Some(body), _permit)) => {
            (
                [
                    (header::CONTENT_TYPE, "application/json; charset=utf-8"),
                    (header::CONTENT_DISPOSITION, "attachment; filename=\"termian-snapshot.json\""),
                ],
                body,
            )
                .into_response()
        }
        Ok(_) => error(StatusCode::NOT_FOUND, "На сервере ещё нет снимка"),
        Err(response) => response,
    }
}

#[derive(Deserialize)]
struct Rotation {
    passphrase: String,
    new_passphrase: String,
    revision: String,
    confirmed: bool,
}

async fn rotate_password(State(state): State<Shared>, peer: Peer, headers: HeaderMap, body: Bytes) -> Response {
    let request: Rotation = match read_request(&headers, &body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    if !request.confirmed || request.new_passphrase.is_empty()
        || request.new_passphrase.len() > 4096 || request.new_passphrase == request.passphrase
    {
        return error(StatusCode::BAD_REQUEST, "Подтвердите замену и введите новый пароль (до 4096 байт).");
    }
    let ip = peer_ip(peer);
    if let Err(response) = require_panel_token(&state, ip, &headers) {
        return response;
    }
    let permit = match decrypt_slot(&state) {
        Ok(permit) => permit,
        Err(response) => return response,
    };
    if let Some(left) = state.panel_limiter.locked(ip) {
        return too_many_attempts(left);
    }
    let wrong_credentials = || match state.panel_limiter.fail(ip) {
        Some(lockout) => too_many_attempts(lockout),
        None => error(StatusCode::UNAUTHORIZED, "Неверный токен или текущий пароль шифрования"),
    };
    let working = state.clone();
    match tokio::task::spawn_blocking(move || (rotate(&working, &request), permit)).await {
        Ok((Ok(revision), _permit)) => {
            state.panel_limiter.succeed(ip);
            Json(json!({"revision": revision})).into_response()
        }
        Ok((Err("passphrase"), _permit)) => wrong_credentials(),
        Ok((Err("revision"), _permit)) => error(StatusCode::CONFLICT, "Снимок изменился. Обновите панель и повторите."),
        Ok((Err("missing"), _permit)) => error(StatusCode::NOT_FOUND, "На сервере ещё нет снимка"),
        Ok((Err("format" | "log"), _permit)) => error(StatusCode::UNPROCESSABLE_ENTITY, "Снимок или журнал нельзя расшифровать текущим паролем. Данные не изменены."),
        _ => error(StatusCode::INTERNAL_SERVER_ERROR, "Не удалось завершить смену пароля. Проверьте снимок перед повтором."),
    }
}

fn panel_scope(cfg: &crate::Config) -> std::io::Result<crate::Scope> {
    let scopes = crate::storage_scopes(cfg)?;
    for scope in scopes.iter().filter(|scope| **scope != crate::Scope::Catalog).chain(scopes.iter().filter(|scope| **scope == crate::Scope::Catalog)) {
        match fs::metadata(snapshot_path(&scope.scoped(cfg))) {
            Ok(_) => return Ok(scope.clone()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(crate::Scope::Personal)
}

fn rotate(state: &Shared, request: &Rotation) -> Result<String, &'static str> {
    use crate::{active_config, durable_replace, logs_dir, storage_scopes, sync_directory, ACTIVE_STORAGE, GENERATION_PREFIX, WORKSPACES};
    let _guard = state.write_lock.lock().map_err(|_| "storage")?;
    let cfg = active_config(&state.cfg).map_err(|_| "storage")?;
    let selected = panel_scope(&cfg).map_err(|_| "storage")?;
    let original = current(&selected.scoped(&cfg)).map_err(|_| "storage")?.ok_or("missing")?;
    if crate::digest(&original) != request.revision {
        return Err("revision");
    }

    let stage = tempfile::Builder::new().prefix(GENERATION_PREFIX)
        .tempdir_in(&state.cfg.dir).map_err(|_| "storage")?;
    let staged_cfg = crate::Config { dir: stage.path().to_owned(), ..cfg.clone() };
    let scopes = storage_scopes(&cfg).map_err(|_| "storage")?;
    let mut replaced_files = Vec::new();
    let mut revision = None;
    for scope in &scopes {
        let staged = stage_rotation(&scope.scoped(&cfg), &scope.scoped(&staged_cfg), request, &mut replaced_files)?;
        if *scope == selected {
            revision = staged;
        }
    }
    let revision = revision.ok_or("missing")?;
    if stage.path().join(WORKSPACES).is_dir() {
        sync_directory(&stage.path().join(WORKSPACES)).map_err(|_| "storage")?;
    }
    sync_directory(stage.path()).map_err(|_| "storage")?;
    sync_directory(&state.cfg.dir).map_err(|_| "storage")?;
    let staged = stage.keep();
    let name = staged.file_name().and_then(|name| name.to_str()).ok_or("storage")?;
    durable_replace(&state.cfg.dir, &state.cfg.dir.join(ACTIVE_STORAGE), name.as_bytes()).map_err(|_| "storage")?;
    for file in replaced_files {
        if fs::remove_file(&file).is_err() {
            eprintln!("Could not remove superseded encrypted data after password rotation");
        }
    }
    for scope in scopes {
        let old = scope.scoped(&cfg);
        for dir in [logs_dir(&old), old.dir.join("revisions")] {
            if dir.is_dir() {
                let _ = sync_directory(&dir);
                let _ = fs::remove_dir(dir);
            }
        }
        let _ = sync_directory(&old.dir);
        if scope != crate::Scope::Personal {
            let _ = fs::remove_dir(&old.dir);
        }
    }
    let _ = fs::remove_dir(cfg.dir.join(WORKSPACES));
    let _ = sync_directory(&cfg.dir);
    if cfg.dir != state.cfg.dir { let _ = fs::remove_dir(&cfg.dir); }
    let _ = sync_directory(&state.cfg.dir);
    Ok(revision)
}

fn stage_rotation(
    cfg: &crate::Config,
    dest: &crate::Config,
    request: &Rotation,
    replaced: &mut Vec<std::path::PathBuf>,
) -> Result<Option<String>, &'static str> {
    use crate::{durable_replace, logs_dir, sync_directory};
    fs::create_dir_all(&dest.dir).map_err(|_| "storage")?;
    let revision = if let Some(original) = current(cfg).map_err(|_| "storage")? {
        let snapshot = reencrypt(&original, request, cfg.limit)?;
        durable_replace(&dest.dir, &snapshot_path(dest), &snapshot).map_err(|_| "storage")?;
        replaced.push(snapshot_path(cfg));
        Some(crate::digest(&snapshot))
    } else {
        None
    };
    let source_logs = logs_dir(cfg);
    if source_logs.is_dir() {
        let dir = logs_dir(dest);
        fs::create_dir(&dir).map_err(|_| "storage")?;
        for entry in fs::read_dir(&source_logs).map_err(|_| "storage")? {
            let entry = entry.map_err(|_| "storage")?;
            if !crate::valid_log_name(&entry.file_name().to_string_lossy()) {
                continue;
            }
            let blob = fs::read(entry.path()).map_err(|_| "storage")?;
            let encrypted = reencrypt(&blob, request, cfg.limit).map_err(|_| "log")?;
            let path = dir.join(entry.file_name());
            durable_replace(&dir, &path, &encrypted).map_err(|_| "storage")?;
            preserve_modified(&entry, path).map_err(|_| "storage")?;
            replaced.push(entry.path());
        }
        sync_directory(&dir).map_err(|_| "storage")?;
    }
    let old_revisions = cfg.dir.join("revisions");
    if old_revisions.is_dir() {
        let dir = dest.dir.join("revisions");
        fs::create_dir(&dir).map_err(|_| "storage")?;
        for entry in fs::read_dir(&old_revisions).map_err(|_| "storage")? {
            let entry = entry.map_err(|_| "storage")?;
            if entry.path().extension().is_some_and(|ext| ext == "snapshot") {
                let path = dir.join(entry.file_name());
                if !entry.file_type().map_err(|_| "storage")?.is_file() { return Err("storage"); }
                let bytes = fs::read(entry.path()).map_err(|_| "storage")?;
                durable_replace(&dir, &path, &bytes).map_err(|_| "storage")?;
                if fs::read(&path).map_err(|_| "storage")? != bytes { return Err("storage"); }
                preserve_modified(&entry, path).map_err(|_| "storage")?;
                replaced.push(entry.path());
            }
        }
        sync_directory(&dir).map_err(|_| "storage")?;
    }
    sync_directory(&dest.dir).map_err(|_| "storage")?;
    Ok(revision)
}

fn preserve_modified(entry: &fs::DirEntry, path: std::path::PathBuf) -> std::io::Result<()> {
    let file = fs::File::options().write(true).open(path)?;
    file.set_times(fs::FileTimes::new().set_modified(entry.metadata()?.modified()?))?;
    file.sync_all()
}

fn reencrypt(data: &[u8], request: &Rotation, limit: usize) -> Result<Vec<u8>, &'static str> {
    let snapshot = open_envelope(data, &request.passphrase)?;
    let mut salt = [0; 16];
    let mut iv = [0; 12];
    getrandom::getrandom(&mut salt).map_err(|_| "storage")?;
    getrandom::getrandom(&mut iv).map_err(|_| "storage")?;
    let mut key = [0; 32];
    let params = scrypt::Params::new(14, 8, 1, 32).map_err(|_| "storage")?;
    scrypt::scrypt(request.new_passphrase.as_bytes(), &salt, &params, &mut key).map_err(|_| "storage")?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| "storage")?;
    let plain = serde_json::to_vec(&snapshot).map_err(|_| "storage")?;
    let mut ciphertext = cipher.encrypt(Nonce::from_slice(&iv), plain.as_slice()).map_err(|_| "storage")?;
    let tag = ciphertext.split_off(ciphertext.len() - 16);
    let envelope = serde_json::to_vec(&json!({
        "version": ENVELOPE_VERSION, "salt": STANDARD.encode(salt), "iv": STANDARD.encode(iv),
        "ciphertext": STANDARD.encode(ciphertext), "tag": STANDARD.encode(tag),
    })).map_err(|_| "storage")?;
    if envelope.len() > limit {
        return Err("storage");
    }
    Ok(envelope)
}

impl AuthLimiter {
    fn locked_clients(&self) -> usize {
        let now = std::time::Instant::now();
        self.clients
            .lock()
            .map(|clients| clients.values().filter(|entry| entry.locked_until.is_some_and(|until| until > now)).count())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_loopback_hosts_are_accepted() {
        for host in ["127.0.0.1:8081", "localhost:8081", "LOCALHOST", "[::1]:8081", "127.0.0.1"] {
            assert!(is_loopback_host(host), "{host}");
        }
        for host in ["evil.example:8081", "127.0.0.1.evil.example", "192.168.1.5:8081", ""] {
            assert!(!is_loopback_host(host), "{host}");
        }
    }

    #[test]
    fn allowed_hosts_extend_loopback() {
        let default = AllowedHosts::parse("");
        assert!(default.allows("127.0.0.1:8081"));
        assert!(!default.allows("192.168.1.5:8081"));
        let listed = AllowedHosts::parse(" 192.168.1.5, Sync.Example.com ");
        assert!(listed.allows("192.168.1.5:8081"));
        assert!(listed.allows("sync.example.com"));
        assert!(listed.allows("localhost:8081"));
        assert!(!listed.allows("evil.example:8081"));
        let any = AllowedHosts::parse("*");
        assert!(any.allows("10.0.0.7:8081") && any.allows(""));
    }
}
