mod panel;

use axum::{
    body::Bytes,
    extract::{ConnectInfo, DefaultBodyLimit, Path as UrlPath, Query, Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    env, fs,
    fs::OpenOptions,
    io::{self, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};
use subtle::ConstantTimeEq;
use tower_http::limit::RequestBodyLimitLayer;

const DEFAULT_LIMIT: usize = 64 * 1024 * 1024;
const MAX_BACKUPS: usize = 10;
const LOG_GENERATION_HEADER: &str = "x-termian-key-generation";
const INITIAL_GENERATION: &str = "initial";
const CONDITIONAL_CREATE_HEADER: &str = "x-termian-conditional-create";
const DEFAULT_LOG_RETENTION_DAYS: u64 = 60;
const LOG_RETENTION_CHECK: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Clone)]
struct Config {
    dir: PathBuf,
    token: Vec<u8>,
    limit: usize,
    /// Session logs older than this (by last upload) are deleted.
    log_retention: Duration,
}

struct AppState {
    cfg: Config,
    write_lock: Mutex<()>,
    limiter: AuthLimiter,
    /// Wrong encryption passwords entered in the local web panel.
    panel_limiter: AuthLimiter,
    panel_tokens: AuthLimiter,
    panel_decrypt: Arc<tokio::sync::Semaphore>,
    started: Instant,
    /// Last successful snapshot download by a client (since start).
    last_download: Mutex<Option<SystemTime>>,
}

impl AppState {
    fn new(cfg: Config, limiter: AuthLimiter) -> Self {
        let panel_limiter = AuthLimiter::new(limiter.free_failures, limiter.base_lockout, false);
        Self {
            cfg,
            write_lock: Mutex::new(()),
            limiter,
            panel_tokens: AuthLimiter::new(panel_limiter.free_failures, panel_limiter.base_lockout, false),
            panel_limiter,
            panel_decrypt: Arc::new(tokio::sync::Semaphore::new(1)),
            started: Instant::now(),
            last_download: Mutex::new(None),
        }
    }
}

/// Brute-force protection for the bearer token. Each client address may fail
/// `free_failures` times; every further failure locks it out for an
/// exponentially growing period (capped at `max_lockout`). While locked out
/// even a correct token is refused, so guesses cannot be confirmed.
/// The server never sees the encryption passphrase: wrong passphrases are
/// throttled by the client, which is the only side able to detect them.
struct AuthLimiter {
    free_failures: u32,
    base_lockout: Duration,
    max_lockout: Duration,
    trust_forwarded_for: bool,
    clients: Mutex<HashMap<IpAddr, Attempts>>,
}

#[derive(Clone, Copy)]
struct Attempts {
    failures: u32,
    locked_until: Option<Instant>,
    last_failure: Instant,
}

impl AuthLimiter {
    fn new(free_failures: u32, base_lockout: Duration, trust_forwarded_for: bool) -> Self {
        Self {
            free_failures,
            base_lockout,
            max_lockout: Duration::from_secs(3600),
            trust_forwarded_for,
            clients: Mutex::new(HashMap::new()),
        }
    }

    /// Remaining lockout for a client, if any.
    fn locked(&self, ip: IpAddr) -> Option<Duration> {
        let clients = self.clients.lock().ok()?;
        let until = clients.get(&ip)?.locked_until?;
        until.checked_duration_since(Instant::now()).filter(|left| !left.is_zero())
    }

    /// Record a failed attempt; returns the lockout it triggered, if any.
    fn fail(&self, ip: IpAddr) -> Option<Duration> {
        let mut clients = self.clients.lock().ok()?;
        let now = Instant::now();
        // Forget idle, unlocked clients so the table cannot grow without bound.
        let forget_after = self.max_lockout;
        clients.retain(|_, entry| {
            entry.locked_until.is_some_and(|until| until > now)
                || now.duration_since(entry.last_failure) < forget_after
        });
        let entry = clients.entry(ip).or_insert(Attempts {
            failures: 0,
            locked_until: None,
            last_failure: now,
        });
        entry.failures = entry.failures.saturating_add(1);
        entry.last_failure = now;
        if entry.failures <= self.free_failures {
            return None;
        }
        let exponent = (entry.failures - self.free_failures - 1).min(16);
        let lockout = self
            .base_lockout
            .saturating_mul(1 << exponent)
            .min(self.max_lockout);
        entry.locked_until = Some(now + lockout);
        Some(lockout)
    }

    fn succeed(&self, ip: IpAddr) {
        if let Ok(mut clients) = self.clients.lock() {
            clients.remove(&ip);
        }
    }

    fn client_ip(&self, request: &Request) -> IpAddr {
        if self.trust_forwarded_for {
            let forwarded = request
                .headers()
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.split(',').next())
                .and_then(|value| value.trim().parse().ok());
            if let Some(ip) = forwarded {
                return ip;
            }
        }
        request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|info| info.0.ip())
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
    }
}

type Shared = Arc<AppState>;

#[tokio::main]
async fn main() {
    let token = env::var("SYNC_BEARER_TOKEN").expect("SYNC_BEARER_TOKEN is required");
    validate_token(&token).expect("invalid SYNC_BEARER_TOKEN");
    let dir = PathBuf::from(env::var("SYNC_DATA_DIR").unwrap_or_else(|_| "./data".into()));
    fs::create_dir_all(&dir).expect("create data directory");
    let limit = env::var("SYNC_MAX_BODY_BYTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_LIMIT);
    let free_failures = env::var("SYNC_AUTH_MAX_FAILURES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let lockout = env::var("SYNC_AUTH_LOCKOUT_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|v| *v > 0)
        .unwrap_or(30);
    let trust_forwarded_for = env::var("SYNC_TRUST_FORWARDED_FOR").is_ok_and(|v| v == "1" || v == "true");
    let log_retention_days = env::var("SYNC_LOG_RETENTION_DAYS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_LOG_RETENTION_DAYS);
    let state = Arc::new(AppState::new(
        Config {
            dir,
            token: token.into_bytes(),
            limit,
            log_retention: Duration::from_secs(log_retention_days * 24 * 60 * 60),
        },
        AuthLimiter::new(free_failures, Duration::from_secs(lockout), trust_forwarded_for),
    ));
    // Web panel: loopback only by default (see SYNC_PANEL_ALLOWED_HOSTS); "off" disables it.
    let panel_addr = env::var("SYNC_PANEL_BIND").unwrap_or_else(|_| "127.0.0.1:8081".into());
    if !matches!(panel_addr.as_str(), "" | "off") {
        let panel_app = panel::make_panel(state.clone());
        let panel_listener = tokio::net::TcpListener::bind(&panel_addr).await.expect("bind panel");
        tokio::spawn(async move {
            axum::serve(panel_listener, panel_app.into_make_service_with_connect_info::<SocketAddr>()).await.expect("panel server");
        });
    }
    let retention = state.clone();
    tokio::spawn(async move {
        loop {
            let state = retention.clone();
            let _ = tokio::task::spawn_blocking(move || {
                with_storage(&state, |cfg| Ok(prune_logs(cfg, SystemTime::now())))
            }).await;
            tokio::time::sleep(LOG_RETENTION_CHECK).await;
        }
    });
    let app = make_app(state);
    let addr = env::var("SYNC_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind");
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .expect("server");
}

fn make_app(state: Shared) -> Router {
    let protected = Router::new()
        .route("/v1/snapshot", get(get_snapshot).put(put_snapshot))
        .route("/v1/logs", axum::routing::delete(delete_logs))
        .route("/v1/logs/:name", get(get_log).put(put_log))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));
    Router::new()
        .route("/health", get(health))
        .merge(protected)
        .with_state(state.clone())
        .layer(DefaultBodyLimit::max(state.cfg.limit))
        .layer(RequestBodyLimitLayer::new(state.cfg.limit))
}

fn validate_token(token: &str) -> Result<(), &'static str> {
    if token.chars().count() < 32 {
        return Err("SYNC_BEARER_TOKEN must contain at least 32 characters");
    }
    if token.starts_with("replace-") || token.chars().all(|c| c == '0') {
        return Err("SYNC_BEARER_TOKEN must not be a placeholder");
    }
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn require_auth(State(state): State<Shared>, request: Request, next: Next) -> Response {
    let ip = state.limiter.client_ip(&request);
    if let Some(left) = state.limiter.locked(ip) {
        return too_many_attempts(left);
    }
    if !authorized(request.headers(), &state) {
        return match state.limiter.fail(ip) {
            Some(lockout) => too_many_attempts(lockout),
            None => unauthorized(),
        };
    }
    state.limiter.succeed(ip);
    next.run(request).await
}

fn too_many_attempts(left: Duration) -> Response {
    let seconds = left.as_secs() + u64::from(left.subsec_nanos() > 0);
    let mut headers = HeaderMap::new();
    headers.insert(header::RETRY_AFTER, seconds.max(1).to_string().parse().unwrap());
    (StatusCode::TOO_MANY_REQUESTS, headers, "too many failed authorization attempts\n").into_response()
}

/// Behind a reverse proxy with HTTP Basic auth the client keeps
/// `Authorization` for the proxy and sends the token in `X-Termian-Token`.
const TOKEN_HEADER: &str = "x-termian-token";

fn authorized(headers: &HeaderMap, state: &AppState) -> bool {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let Some(given) = bearer.or_else(|| headers.get(TOKEN_HEADER).and_then(|value| value.to_str().ok())) else {
        return false;
    };
    bool::from(given.as_bytes().ct_eq(&state.cfg.token))
}

fn unauthorized() -> Response {
    (StatusCode::UNAUTHORIZED, "unauthorized\n").into_response()
}

const ACTIVE_STORAGE: &str = ".active-storage";
const GENERATION_PREFIX: &str = "sealed-";

fn active_config(cfg: &Config) -> io::Result<Config> {
    let name = match fs::read_to_string(cfg.dir.join(ACTIVE_STORAGE)) {
        Ok(name) => name,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(cfg.clone()),
        Err(error) => return Err(error),
    };
    if !name.strip_prefix(GENERATION_PREFIX).is_some_and(|id| {
        !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric())
    }) {
        return Err(io::Error::other("invalid active storage directory"));
    }
    let mut active = cfg.clone();
    active.dir = active.dir.join(name);
    if !active.dir.is_dir() {
        return Err(io::Error::other("active storage directory missing"));
    }
    Ok(active)
}

fn with_storage<T>(state: &AppState, f: impl FnOnce(&Config) -> io::Result<T>) -> io::Result<T> {
    let _guard = state.write_lock.lock().map_err(|_| io::Error::other("storage lock poisoned"))?;
    f(&active_config(&state.cfg)?)
}

fn snapshot_path(cfg: &Config) -> PathBuf {
    cfg.dir.join("snapshot.bin")
}

fn digest(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

fn etag(hash: &str) -> String {
    format!("\"{hash}\"")
}

fn current(cfg: &Config) -> io::Result<Option<Vec<u8>>> {
    match fs::read(snapshot_path(cfg)) {
        Ok(data) => Ok(Some(data)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn storage_generation<'a>(state: &AppState, cfg: &'a Config) -> &'a str {
    if cfg.dir == state.cfg.dir {
        INITIAL_GENERATION
    } else {
        cfg.dir.file_name().and_then(|name| name.to_str()).expect("validated storage generation")
    }
}

async fn get_snapshot(State(state): State<Shared>) -> Response {
    let storage = state.clone();
    match tokio::task::spawn_blocking(move || with_storage(&storage, |cfg| Ok((current(cfg)?, storage_generation(&storage, cfg).to_owned())))).await {
        Ok(Ok((Some(data), generation))) => {
            if let Ok(mut last) = state.last_download.lock() {
                *last = Some(SystemTime::now());
            }
            let hash = digest(&data);
            let mut headers = HeaderMap::new();
            headers.insert(LOG_GENERATION_HEADER, generation.parse().unwrap());
            headers.insert(header::ETAG, etag(&hash).parse().unwrap());
            headers.insert(
                header::CONTENT_TYPE,
                "application/octet-stream".parse().unwrap(),
            );
            (StatusCode::OK, headers, data).into_response()
        }
        Ok(Ok((None, generation))) => {
            // Only upgraded servers advertise atomic If-None-Match creation.
            // Older servers ignore that request header; clients must fail
            // closed rather than risk replacing another client's snapshot.
            let mut headers = HeaderMap::new();
            headers.insert(LOG_GENERATION_HEADER, generation.parse().unwrap());
            headers.insert(CONDITIONAL_CREATE_HEADER, "v1".parse().unwrap());
            (StatusCode::NOT_FOUND, headers, "snapshot not found\n").into_response()
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "storage error\n").into_response(),
    }
}

async fn put_snapshot(
    State(state): State<Shared>,
    headers: HeaderMap,
    body: Result<Bytes, axum::extract::rejection::BytesRejection>,
) -> Response {
    let body = match body {
        Ok(body) => body,
        Err(_) => return (StatusCode::PAYLOAD_TOO_LARGE, "payload too large\n").into_response(),
    };
    if body.len() > state.cfg.limit {
        return (StatusCode::PAYLOAD_TOO_LARGE, "payload too large\n").into_response();
    }
    let condition = match headers.get(header::IF_MATCH) {
        Some(value) => match value.to_str() {
            Ok(value) => Some(value.trim().to_owned()),
            Err(_) => {
                return (StatusCode::PRECONDITION_FAILED, "revision mismatch\n").into_response()
            }
        },
        None => None,
    };
    // Creation must be atomic too: an initial GET 404 followed by an
    // unconditional PUT can otherwise erase another client's new snapshot.
    let create_only = match headers.get(header::IF_NONE_MATCH) {
        Some(value) if value.as_bytes() == b"*" && condition.is_none() => true,
        Some(_) => return (StatusCode::PRECONDITION_FAILED, "revision mismatch\n").into_response(),
        None => false,
    };
    let result = tokio::task::spawn_blocking(move || {
        store_snapshot(&state, &body, condition.as_deref(), create_only)
    })
    .await;
    match result {
        Ok(Ok(hash)) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::ETAG, etag(&hash).parse().unwrap());
            (StatusCode::OK, headers, "stored\n").into_response()
        }
        Ok(Err(StoreError::Precondition)) => {
            (StatusCode::PRECONDITION_FAILED, "revision mismatch\n").into_response()
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "storage error\n").into_response(),
    }
}

#[derive(Debug)]
enum StoreError {
    Storage,
    Precondition,
}

impl From<io::Error> for StoreError {
    fn from(_value: io::Error) -> Self {
        Self::Storage
    }
}

fn store_snapshot(
    state: &AppState,
    body: &[u8],
    condition: Option<&str>,
    create_only: bool,
) -> Result<String, StoreError> {
    let _guard = state
        .write_lock
        .lock()
        .map_err(|_| io::Error::other("write lock poisoned"))?;
    let cfg = active_config(&state.cfg)?;
    let old = current(&cfg)?;
    if (storage_generation(state, &cfg) != INITIAL_GENERATION && !create_only && condition.is_none_or(|tag| tag == "*"))
        || (create_only && old.is_some()) || !if_match_satisfied(condition, old.as_deref()) {
        return Err(StoreError::Precondition);
    }
    if let Some(previous) = old.as_deref() {
        backup(&cfg, previous, &digest(previous))?;
    }
    durable_replace(&cfg.dir, &snapshot_path(&cfg), body)?;
    Ok(digest(body))
}

fn if_match_satisfied(condition: Option<&str>, current_data: Option<&[u8]>) -> bool {
    let Some(condition) = condition else {
        return true;
    };
    let Some(current_data) = current_data else {
        return false;
    };
    if condition == "*" {
        return true;
    }
    let expected = etag(&digest(current_data));
    bool::from(condition.as_bytes().ct_eq(expected.as_bytes()))
}

fn durable_replace(dir: &Path, dst: &Path, data: &[u8]) -> io::Result<()> {
    let mut attempt = 0_u32;
    let (tmp_path, mut file) = loop {
        let path = dir.join(format!(".snapshot.{}.{}.tmp", std::process::id(), attempt));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => break (path, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => attempt += 1,
            Err(error) => return Err(error),
        }
    };
    let result = (|| {
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp_path, dst)?;
        sync_directory(dir)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    result
}

fn sync_directory(dir: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        fs::File::open(dir)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
        Ok(())
    }
}

fn backup(cfg: &Config, data: &[u8], hash: &str) -> io::Result<()> {
    let dir = cfg.dir.join("revisions");
    fs::create_dir_all(&dir)?;
    sync_directory(&cfg.dir)?;
    let path = dir.join(format!("{hash}.snapshot"));
    if !path.exists() {
        durable_replace(&dir, &path, data)?;
    }
    let mut files: Vec<_> = fs::read_dir(&dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "snapshot")
        })
        .collect();
    files.sort_by_key(|entry| entry.metadata().and_then(|meta| meta.modified()).ok());
    while files.len() > MAX_BACKUPS {
        fs::remove_file(files.remove(0).path())?;
    }
    sync_directory(&dir)
}

// ---------------------------------------------------------- session logs --
//
// Terminal session logs uploaded by clients in the background. Each one is an
// opaque, client-encrypted blob, independent of the snapshot and its
// revisions, and is deleted after the retention period.

fn valid_log_name(name: &str) -> bool {
    (1..=128).contains(&name.len())
        && !name.starts_with('.')
        && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn logs_dir(cfg: &Config) -> PathBuf {
    cfg.dir.join("logs")
}

async fn get_log(State(state): State<Shared>, UrlPath(name): UrlPath<String>) -> Response {
    if !valid_log_name(&name) {
        return (StatusCode::BAD_REQUEST, "invalid log name\n").into_response();
    }
    match tokio::task::spawn_blocking(move || with_storage(&state, |cfg| fs::read(logs_dir(cfg).join(&name)))).await {
        Ok(Ok(data)) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "application/octet-stream".parse().unwrap());
            (StatusCode::OK, headers, data).into_response()
        }
        Ok(Err(error)) if error.kind() == io::ErrorKind::NotFound => {
            (StatusCode::NOT_FOUND, "log not found\n").into_response()
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "storage error\n").into_response(),
    }
}

async fn put_log(
    State(state): State<Shared>,
    UrlPath(name): UrlPath<String>,
    headers: HeaderMap,
    body: Result<Bytes, axum::extract::rejection::BytesRejection>,
) -> Response {
    if !valid_log_name(&name) {
        return (StatusCode::BAD_REQUEST, "invalid log name\n").into_response();
    }
    let body = match body {
        Ok(body) if body.len() <= state.cfg.limit => body,
        _ => return (StatusCode::PAYLOAD_TOO_LARGE, "payload too large\n").into_response(),
    };
    let supplied = headers.get(LOG_GENERATION_HEADER).cloned();
    let revision = headers.get("x-termian-snapshot-revision").cloned();
    let result = tokio::task::spawn_blocking(move || with_storage(&state, |cfg| {
        let Some(snapshot) = current(cfg)? else { return Ok(false) };
        if !revision.is_some_and(|r| r.as_bytes() == etag(&digest(&snapshot)).as_bytes()) { return Ok(false); }
        let active = storage_generation(&state, cfg);
        if !supplied.as_ref().map_or(active == INITIAL_GENERATION, |value| value.as_bytes() == active.as_bytes()) {
            return Ok(false);
        }
        let dir = logs_dir(cfg);
        fs::create_dir_all(&dir)?;
        durable_replace(&dir, &dir.join(&name), &body)?;
        Ok(true)
    }))
    .await;
    match result {
        Ok(Ok(true)) => (StatusCode::OK, "stored\n").into_response(),
        Ok(Ok(false)) => (StatusCode::PRECONDITION_FAILED, "snapshot revision or key generation mismatch\n").into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "storage error\n").into_response(),
    }
}

#[derive(serde::Deserialize)]
struct ClearLogs {
    /// Unix time in milliseconds; logs uploaded at or after it are deleted.
    /// 0 or missing deletes every log.
    since: Option<u64>,
}

async fn delete_logs(State(state): State<Shared>, Query(query): Query<ClearLogs>) -> Response {
    let since = SystemTime::UNIX_EPOCH + Duration::from_millis(query.since.unwrap_or(0));
    match tokio::task::spawn_blocking(move || with_storage(&state, |cfg| clear_logs(cfg, since))).await {
        Ok(Ok((removed, failed))) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
            (if failed == 0 { StatusCode::OK } else { StatusCode::INTERNAL_SERVER_ERROR }, headers, format!("{{\"removed\":{removed},\"failed\":{failed}}}")).into_response()
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "storage error\n").into_response(),
    }
}

fn clear_logs(cfg: &Config, since: SystemTime) -> io::Result<(usize, usize)> {
    let entries = match fs::read_dir(logs_dir(cfg)) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok((0, 0)),
        Err(error) => return Err(error),
    };
    let (mut removed, mut failed) = (0, 0);
    for entry in entries {
        let result = entry.and_then(|entry| {
            if valid_log_name(&entry.file_name().to_string_lossy()) && entry.metadata()?.modified()? >= since {
                fs::remove_file(entry.path())?;
                removed += 1;
            }
            Ok(())
        });
        if result.is_err() { failed += 1; }
    }
    Ok((removed, failed))
}

/// Delete logs not uploaded within the retention period; returns how many.
fn prune_logs(cfg: &Config, now: SystemTime) -> usize {
    let Ok(entries) = fs::read_dir(logs_dir(cfg)) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| valid_log_name(&entry.file_name().to_string_lossy()))
        .filter(|entry| {
            entry.metadata().and_then(|meta| meta.modified()).ok()
                .and_then(|modified| now.duration_since(modified).ok())
                .is_some_and(|age| age > cfg.log_retention)
        })
        .filter(|entry| fs::remove_file(entry.path()).is_ok())
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tempfile::TempDir;
    use tower::ServiceExt;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";

    fn app(dir: &Path, limit: usize) -> Router {
        make_app(state(dir, limit))
    }

    fn state(dir: &Path, limit: usize) -> Shared {
        Arc::new(AppState::new(
            Config {
                dir: dir.into(),
                token: TOKEN.as_bytes().to_vec(),
                limit,
                log_retention: Duration::from_secs(60 * 24 * 60 * 60),
            },
            AuthLimiter::new(3, Duration::from_millis(200), false),
        ))
    }

    /// Envelope exactly like the Termian client writes it (fixed salt/IV).
    fn seal(snapshot: &serde_json::Value, passphrase: &str) -> Vec<u8> {
        use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        let (salt, iv) = ([7u8; 16], [9u8; 12]);
        let params = scrypt::Params::new(14, 8, 1, 32).unwrap();
        let mut key = [0u8; 32];
        scrypt::scrypt(passphrase.as_bytes(), &salt, &params, &mut key).unwrap();
        let mut sealed = Aes256Gcm::new_from_slice(&key).unwrap()
            .encrypt(Nonce::from_slice(&iv), serde_json::to_vec(snapshot).unwrap().as_slice()).unwrap();
        let tag = sealed.split_off(sealed.len() - 16);
        serde_json::to_vec(&serde_json::json!({"version": 1, "salt": STANDARD.encode(salt),
            "iv": STANDARD.encode(iv), "tag": STANDARD.encode(tag), "ciphertext": STANDARD.encode(sealed)})).unwrap()
    }

    fn panel_request(path: &str, host: &str, passphrase: &str) -> axum::http::Request<Body> {
        axum::http::Request::builder()
            .method("POST")
            .uri(path)
            .header(header::HOST, host)
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-termian-panel", "1")
            .body(Body::from(serde_json::json!({"passphrase": passphrase}).to_string()))
            .unwrap()
    }

    fn panel_export(host: &str, passphrase: &str) -> axum::http::Request<Body> {
        let mut request = panel_request("/api/export", host, passphrase);
        request.headers_mut().insert(TOKEN_HEADER, TOKEN.parse().unwrap());
        request
    }

    async fn json_body(response: Response) -> serde_json::Value {
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
    }

    #[tokio::test]
    async fn panel_unlocks_with_the_encryption_password_and_exports_json() {
        let dir = TempDir::new().unwrap();
        let shared = state(dir.path(), 1024 * 1024);
        let panel = panel::make_panel(shared.clone());
        let host = "127.0.0.1:8081";
        // No snapshot yet: server info only.
        let response = panel.clone().oneshot(panel_request("/api/unlock", host, "anything")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert!(body["snapshot"].is_null());
        assert_eq!(body["server"]["snapshotExists"], false);

        let snapshot = serde_json::json!({"format": "termius-portable-snapshot", "version": 1,
            "createdAt": "2026-11-08T10:00:00.000Z", "hosts": [{"label": "a"}, {"label": "b"}], "groups": [{}], "settings": {"x": "1"}});
        let upload = make_app(shared.clone()).oneshot(request("PUT", Body::from(seal(&snapshot, "correct horse battery")))).await.unwrap();
        assert_eq!(upload.status(), StatusCode::OK);

        let response = panel.clone().oneshot(panel_request("/api/unlock", host, "correct horse battery")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["server"]["snapshotExists"], true);
        assert!(body["server"]["lastUploadAt"].as_u64().unwrap() > 0);
        assert_eq!(body["snapshot"]["createdAt"], "2026-11-08T10:00:00.000Z");
        let hosts = body["snapshot"]["collections"].as_array().unwrap().iter().find(|c| c["key"] == "hosts").unwrap();
        assert_eq!(hosts["count"], 2);
        assert_eq!(body["snapshot"]["settingsCount"], 1);

        let response = panel.clone().oneshot(panel_request("/api/export", host, "correct horse battery")).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let response = panel.clone().oneshot(panel_export("localhost:8081", "correct horse battery")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers()[header::CONTENT_DISPOSITION].to_str().unwrap().contains("attachment"));
        assert_eq!(json_body(response).await, snapshot);
    }

    fn rotation_request(revision: &str, old: &str, token: Option<&str>) -> axum::http::Request<Body> {
        let mut request = axum::http::Request::builder().method("POST").uri("/api/rotate-password")
            .header(header::HOST, "127.0.0.1:8081")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-termian-panel", "1");
        if let Some(token) = token {
            request = request.header("x-termian-token", token);
        }
        request.body(Body::from(serde_json::json!({
            "passphrase": old, "new_passphrase": "7",
            "revision": revision, "confirmed": true,
        }).to_string())).unwrap()
    }

    #[tokio::test]
    async fn rotation_commits_snapshot_and_logs_together() {
        let dir = TempDir::new().unwrap();
        let shared = state(dir.path(), 1024 * 1024);
        let sync = make_app(shared.clone());
        let panel = panel::make_panel(shared.clone());
        let snapshot = serde_json::json!({"version": 1, "hosts": [{"label": "test host"}]});
        let original = seal(&snapshot, "");
        let log = serde_json::json!({"version": 1, "name": "test.log"});
        let bad_log = seal(&log, "mismatched");
        let current_log = seal(&log, "");
        let log_path = logs_dir(&shared.cfg).join("test.log");
        fs::create_dir_all(log_path.parent().unwrap()).unwrap();
        fs::write(&log_path, &bad_log).unwrap();

        let upload = sync.clone().oneshot(request("PUT", Body::from(original.clone()))).await.unwrap();
        assert_eq!(upload.status(), StatusCode::OK);
        let revision = digest(&original);
        let missing = panel.clone().oneshot(rotation_request(&revision, "", None)).await.unwrap();
        assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);
        let stale = panel.clone().oneshot(rotation_request("stale", "", Some(TOKEN))).await.unwrap();
        assert_eq!(stale.status(), StatusCode::CONFLICT);
        let blocked = panel.clone().oneshot(rotation_request(&revision, "", Some(TOKEN))).await.unwrap();
        assert_eq!(blocked.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(current(&active_config(&shared.cfg).unwrap()).unwrap(), Some(original.clone()));
        assert_eq!(fs::read(&log_path).unwrap(), bad_log);

        fs::write(&log_path, current_log).unwrap();
        let modified = fs::metadata(&log_path).unwrap().modified().unwrap();
        let revisions = shared.cfg.dir.join("revisions");
        fs::create_dir_all(&revisions).unwrap();
        fs::write(revisions.join("earlier.snapshot"), &original).unwrap();
        let done = panel.clone().oneshot(rotation_request(&revision, "", Some(TOKEN))).await.unwrap();
        assert_eq!(done.status(), StatusCode::OK);
        let new_revision = json_body(done).await["revision"].as_str().unwrap().to_owned();
        assert_ne!(new_revision, revision);
        assert!(!snapshot_path(&shared.cfg).exists());
        assert!(!log_path.exists());

        let restarted = state(dir.path(), 1024 * 1024);
        let active = active_config(&restarted.cfg).unwrap();
        assert_eq!(fs::read(active.dir.join("revisions/earlier.snapshot")).unwrap(), original);
        assert!(!revisions.join("earlier.snapshot").exists());
        let log_path = logs_dir(&active).join("test.log");
        let stored = fs::read(&log_path).unwrap();
        assert_eq!(fs::metadata(&log_path).unwrap().modified().unwrap(), modified);
        assert_eq!(panel::open_envelope(&stored, "7").unwrap(), log);
        assert!(panel::open_envelope(&stored, "").is_err());

        let sync = make_app(restarted.clone());
        let downloaded = sync.clone().oneshot(request("GET", Body::empty())).await.unwrap();
        assert_eq!(downloaded.headers()[header::ETAG], etag(&new_revision));
        let generation = downloaded.headers()[LOG_GENERATION_HEADER].clone();
        for stale in [None, Some(header::HeaderValue::from_static(INITIAL_GENERATION))] {
            let mut log = log_request("PUT", "old.log", Body::from(stored.clone()));
            if let Some(header) = stale {
                log.headers_mut().insert(LOG_GENERATION_HEADER, header);
            }
            assert_eq!(sync.clone().oneshot(log).await.unwrap().status(), StatusCode::PRECONDITION_FAILED);
        }
        let mut log = log_request("PUT", "current.log", Body::from(stored));
        log.headers_mut().insert(LOG_GENERATION_HEADER, generation);
        log.headers_mut().insert("x-termian-snapshot-revision", etag(&new_revision).parse().unwrap());
        assert_eq!(sync.clone().oneshot(log).await.unwrap().status(), StatusCode::OK);
        let bytes = downloaded.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(panel::open_envelope(&bytes, "7").unwrap(), snapshot);
        let unconditional = request("PUT", Body::from(original.clone()));
        assert_eq!(sync.clone().oneshot(unconditional).await.unwrap().status(), StatusCode::PRECONDITION_FAILED);
        let mut wildcard = request("PUT", Body::from(original.clone()));
        wildcard.headers_mut().insert(header::IF_MATCH, "*".parse().unwrap());
        assert_eq!(sync.clone().oneshot(wildcard).await.unwrap().status(), StatusCode::PRECONDITION_FAILED);
        let mut stale_upload = request("PUT", Body::from(original));
        stale_upload.headers_mut().insert(header::IF_MATCH, etag(&revision).parse().unwrap());
        assert_eq!(sync.oneshot(stale_upload).await.unwrap().status(), StatusCode::PRECONDITION_FAILED);
    }

    #[tokio::test]
    async fn panel_accepts_empty_short_and_long_passphrases() {
        let snapshot = serde_json::json!({"version": 1});
        let long = "  密碼🔑!  ".repeat(1024);
        let host = "127.0.0.1:8081";

        for passphrase in ["", "7", long.as_str()] {
            let dir = TempDir::new().unwrap();
            let shared = state(dir.path(), 1024 * 1024);
            let upload = make_app(shared.clone())
                .oneshot(request("PUT", Body::from(seal(&snapshot, passphrase))))
                .await
                .unwrap();
            assert_eq!(upload.status(), StatusCode::OK);

            let panel = panel::make_panel(shared);
            let response = panel.clone().oneshot(panel_request("/api/unlock", host, passphrase)).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK, "passphrase length: {}", passphrase.len());
            let response = panel.clone().oneshot(panel_request("/api/export", host, passphrase)).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            let response = panel.oneshot(panel_export(host, passphrase)).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(json_body(response).await, snapshot);
        }
    }

    #[tokio::test]
    async fn panel_limits_token_failures_across_export_and_rotation() {
        let dir = TempDir::new().unwrap();
        let shared = state(dir.path(), 1024 * 1024);
        let panel = panel::make_panel(shared.clone());
        let host = "localhost";
        for _ in 0..shared.panel_tokens.free_failures {
            let mut request = panel_export(host, "");
            request.headers_mut().insert(TOKEN_HEADER, "incorrect".parse().unwrap());
            assert_eq!(panel.clone().oneshot(request).await.unwrap().status(), StatusCode::UNAUTHORIZED);
        }
        let blocked = panel.clone().oneshot(panel_request("/api/export", host, "")).await.unwrap();
        assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(blocked.headers().contains_key(header::RETRY_AFTER));
        assert_eq!(panel.clone().oneshot(panel_export(host, "")).await.unwrap().status(), StatusCode::TOO_MANY_REQUESTS);
        let rotation = rotation_request("revision", "", Some(TOKEN));
        assert_eq!(panel.oneshot(rotation).await.unwrap().status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn panel_defers_decryption_when_the_slot_is_occupied() {
        let dir = TempDir::new().unwrap();
        let shared = state(dir.path(), 1024 * 1024);
        let panel = panel::make_panel(shared.clone());
        let slot = shared.panel_decrypt.clone().try_acquire_owned().unwrap();
        let response = panel.clone().oneshot(panel_request("/api/unlock", "localhost", "")).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()[header::RETRY_AFTER], "1");
        drop(slot);
        let response = panel.oneshot(panel_request("/api/unlock", "localhost", "")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn panel_rejects_wrong_passwords_with_lockout_and_foreign_hosts() {
        let dir = TempDir::new().unwrap();
        let shared = state(dir.path(), 1024 * 1024);
        let snapshot = serde_json::json!({"format": "termius-portable-snapshot", "version": 1});
        make_app(shared.clone()).oneshot(request("PUT", Body::from(seal(&snapshot, "correct horse battery")))).await.unwrap();
        let panel = panel::make_panel(shared);
        for passphrase in ["", "wrong", "wrong"] {
            let response = panel.clone().oneshot(panel_export("127.0.0.1:8081", passphrase)).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
        let response = panel.clone().oneshot(panel_export("127.0.0.1:8081", "wrong")).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let response = panel.clone().oneshot(panel_request("/api/unlock", "127.0.0.1:8081", "correct horse battery")).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS, "lockout applies to the right password too");

        // DNS rebinding: a foreign Host never reaches the API, even the page.
        let response = panel.clone().oneshot(panel_request("/api/unlock", "attacker.example", "x")).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        // Cross-site form posts lack the JSON body and custom header.
        let form = axum::http::Request::builder().method("POST").uri("/api/export").header(header::HOST, "127.0.0.1:8081")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded").body(Body::from("passphrase=x")).unwrap();
        assert_eq!(panel.clone().oneshot(form).await.unwrap().status(), StatusCode::BAD_REQUEST);
        let page = axum::http::Request::builder().uri("/").header(header::HOST, "localhost:8081").body(Body::empty()).unwrap();
        let response = panel.oneshot(page).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers()[header::CONTENT_SECURITY_POLICY].to_str().unwrap().contains("frame-ancestors 'none'"));
    }

    fn request_from(ip: &str, token: &str) -> axum::http::Request<Body> {
        let mut request = axum::http::Request::builder()
            .method("GET")
            .uri("/v1/snapshot")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let addr: SocketAddr = format!("{ip}:40000").parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(addr));
        request
    }

    #[tokio::test]
    async fn repeated_wrong_tokens_lock_out_the_client_with_retry_after() {
        let dir = TempDir::new().unwrap();
        let app = app(dir.path(), 1024);
        let wrong = "wrong-token-wrong-token-wrong-token";
        for _ in 0..3 {
            let response = app.clone().oneshot(request_from("10.0.0.7", wrong)).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
        let response = app.clone().oneshot(request_from("10.0.0.7", wrong)).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key(header::RETRY_AFTER));
        // A correct token cannot confirm a guess while the lockout lasts.
        let response = app.clone().oneshot(request_from("10.0.0.7", TOKEN)).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        // Other clients are unaffected.
        let response = app.clone().oneshot(request_from("10.0.0.8", TOKEN)).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        tokio::time::sleep(Duration::from_millis(250)).await;
        let response = app.clone().oneshot(request_from("10.0.0.7", TOKEN)).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        // Success resets the counter: three new failures are allowed again.
        for _ in 0..3 {
            let response = app.clone().oneshot(request_from("10.0.0.7", wrong)).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
    }

    #[tokio::test]
    async fn lockout_grows_exponentially() {
        let limiter = AuthLimiter::new(1, Duration::from_secs(30), false);
        let ip: IpAddr = "192.0.2.1".parse().unwrap();
        assert!(limiter.fail(ip).is_none());
        assert_eq!(limiter.fail(ip), Some(Duration::from_secs(30)));
        assert_eq!(limiter.fail(ip), Some(Duration::from_secs(60)));
        assert_eq!(limiter.fail(ip), Some(Duration::from_secs(120)));
        for _ in 0..20 { limiter.fail(ip); }
        assert_eq!(limiter.fail(ip), Some(Duration::from_secs(3600)));
    }

    fn request(method: &str, body: Body) -> axum::http::Request<Body> {
        axum::http::Request::builder()
            .method(method)
            .uri("/v1/snapshot")
            .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
            .body(body)
            .unwrap()
    }

    #[tokio::test]
    async fn auth_and_missing() {
        let dir = TempDir::new().unwrap();
        let app = app(dir.path(), 10);
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/v1/snapshot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let response = app.oneshot(request("GET", Body::empty())).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(CONDITIONAL_CREATE_HEADER).unwrap(),
            "v1"
        );
    }

    #[tokio::test]
    async fn token_header_behind_basic_auth_proxy() {
        let dir = TempDir::new().unwrap();
        let app = app(dir.path(), 10);
        let with = |token: &str| {
            axum::http::Request::builder()
                .uri("/v1/snapshot")
                .header(header::AUTHORIZATION, "Basic YWxpY2U6czNjcmV0")
                .header(TOKEN_HEADER, token)
                .body(Body::empty())
                .unwrap()
        };
        let response = app.clone().oneshot(with(TOKEN)).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let response = app.oneshot(with("wrong")).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn unconditional_overwrite_and_if_match() {
        let dir = TempDir::new().unwrap();
        let app = app(dir.path(), 1024);
        let first = app
            .clone()
            .oneshot(request("PUT", Body::from("first")))
            .await
            .unwrap();
        let first_etag = first.headers()[header::ETAG].clone();
        assert_eq!(first.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(request("PUT", Body::from("unconditional")))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let mut mismatch = request("PUT", Body::from("rejected"));
        mismatch.headers_mut().insert(header::IF_MATCH, first_etag);
        let response = app.clone().oneshot(mismatch).await.unwrap();
        assert_eq!(response.status(), StatusCode::PRECONDITION_FAILED);

        let mut wildcard = request("PUT", Body::from("wildcard"));
        wildcard
            .headers_mut()
            .insert(header::IF_MATCH, "*".parse().unwrap());
        let response = app.clone().oneshot(wildcard).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app.oneshot(request("GET", Body::empty())).await.unwrap();
        assert_eq!(
            response.into_body().collect().await.unwrap().to_bytes(),
            &b"wildcard"[..]
        );
    }

    #[tokio::test]
    async fn if_none_match_star_creates_once_without_overwriting_a_concurrent_snapshot() {
        let dir = TempDir::new().unwrap();
        let app = app(dir.path(), 1024);
        let mut first = request("PUT", Body::from("first"));
        first
            .headers_mut()
            .insert(header::IF_NONE_MATCH, "*".parse().unwrap());
        assert_eq!(
            app.clone().oneshot(first).await.unwrap().status(),
            StatusCode::OK
        );
        let mut second = request("PUT", Body::from("should-not-replace"));
        second
            .headers_mut()
            .insert(header::IF_NONE_MATCH, "*".parse().unwrap());
        assert_eq!(
            app.clone().oneshot(second).await.unwrap().status(),
            StatusCode::PRECONDITION_FAILED
        );
        let response = app
            .clone()
            .oneshot(request("GET", Body::empty()))
            .await
            .unwrap();
        assert_eq!(
            response.into_body().collect().await.unwrap().to_bytes(),
            &b"first"[..]
        );
        let mut conflicting = request("PUT", Body::from("conflict"));
        conflicting
            .headers_mut()
            .insert(header::IF_NONE_MATCH, "*".parse().unwrap());
        conflicting
            .headers_mut()
            .insert(header::IF_MATCH, "*".parse().unwrap());
        assert_eq!(
            app.clone().oneshot(conflicting).await.unwrap().status(),
            StatusCode::PRECONDITION_FAILED
        );
        let mut unsupported = request("PUT", Body::from("bad"));
        unsupported
            .headers_mut()
            .insert(header::IF_NONE_MATCH, "anything".parse().unwrap());
        assert_eq!(
            app.oneshot(unsupported).await.unwrap().status(),
            StatusCode::PRECONDITION_FAILED
        );
    }

    #[tokio::test]
    async fn if_match_requires_an_existing_snapshot() {
        let dir = TempDir::new().unwrap();
        let app = app(dir.path(), 1024);
        let mut put = request("PUT", Body::from("new"));
        put.headers_mut()
            .insert(header::IF_MATCH, "*".parse().unwrap());
        let response = app.oneshot(put).await.unwrap();
        assert_eq!(response.status(), StatusCode::PRECONDITION_FAILED);
    }

    #[tokio::test]
    async fn accepts_payload_larger_than_axum_default_and_enforces_configured_limit() {
        let dir = TempDir::new().unwrap();
        let app = app(dir.path(), 3 * 1024 * 1024);
        let large = vec![7_u8; 2 * 1024 * 1024 + 1];
        let response = app
            .clone()
            .oneshot(request("PUT", Body::from(large)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let too_large = vec![0_u8; 3 * 1024 * 1024 + 1];
        let response = app
            .oneshot(request("PUT", Body::from(too_large)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn auth_rejects_before_reading_body() {
        let dir = TempDir::new().unwrap();
        let app = app(dir.path(), 1024);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/v1/snapshot")
                    .body(Body::from(vec![0_u8; 2048]))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn concurrent_writes_use_distinct_temporary_files() {
        let dir = TempDir::new().unwrap();
        let app = app(dir.path(), 1024);
        let first = tokio::spawn(app.clone().oneshot(request("PUT", Body::from("one"))));
        let second = tokio::spawn(app.clone().oneshot(request("PUT", Body::from("two"))));
        assert_eq!(first.await.unwrap().unwrap().status(), StatusCode::OK);
        assert_eq!(second.await.unwrap().unwrap().status(), StatusCode::OK);
        assert!(!dir.path().read_dir().unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")
        }));
    }

    fn log_request(method: &str, name: &str, body: Body) -> axum::http::Request<Body> {
        axum::http::Request::builder()
            .method(method)
            .uri(format!("/v1/logs/{name}"))
            .header("x-termian-snapshot-revision", etag(&digest(b"snapshot")))
            .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
            .body(body)
            .unwrap()
    }

    #[tokio::test]
    async fn log_cleanup_reports_partial_failure() {
        let dir = TempDir::new().unwrap();
        let shared = state(dir.path(), 1024);
        let logs = logs_dir(&shared.cfg);
        fs::create_dir_all(logs.join("blocked.log")).unwrap();
        fs::write(logs.join("ok.log"), b"test").unwrap();
        let mut req = request("DELETE", Body::empty());
        *req.uri_mut() = "/v1/logs".parse().unwrap();
        let response = make_app(shared).oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json_body(response).await, serde_json::json!({"removed": 1, "failed": 1}));
    }

    #[tokio::test]
    async fn session_logs_are_stored_apart_from_the_snapshot() {
        let dir = TempDir::new().unwrap();
        let app = app(dir.path(), 1024);
        let response = app.clone().oneshot(log_request("GET", "a-1.log", Body::empty())).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(app.clone().oneshot(log_request("PUT", "a-1.log", Body::from("log"))).await.unwrap().status(), StatusCode::PRECONDITION_FAILED);
        app.clone().oneshot(request("PUT", Body::from("snapshot"))).await.unwrap();
        for revision in [None, Some("stale")] {
            let mut req = log_request("PUT", "a-1.log", Body::from("log"));
            req.headers_mut().remove("x-termian-snapshot-revision");
            if let Some(value) = revision { req.headers_mut().insert("x-termian-snapshot-revision", value.parse().unwrap()); }
            assert_eq!(app.clone().oneshot(req).await.unwrap().status(), StatusCode::PRECONDITION_FAILED);
        }
        for body in ["first", "second"] {
            let response = app.clone().oneshot(log_request("PUT", "a-1.log", Body::from(body))).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
        let response = app.clone().oneshot(log_request("GET", "a-1.log", Body::empty())).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.into_body().collect().await.unwrap().to_bytes(), &b"second"[..]);
        let response = app.clone().oneshot(request("GET", Body::empty())).await.unwrap();
        assert_eq!(response.headers()[header::ETAG], etag(&digest(b"snapshot")));
        assert!(!dir.path().join("revisions").exists());

        let response = app.clone().oneshot(log_request("PUT", ".hidden", Body::from("x"))).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let response = app.clone().oneshot(log_request("PUT", "big.log", Body::from(vec![0u8; 2048]))).await.unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let unauthorized = axum::http::Request::builder().uri("/v1/logs/a-1.log").body(Body::empty()).unwrap();
        assert_eq!(app.clone().oneshot(unauthorized).await.unwrap().status(), StatusCode::UNAUTHORIZED);

        // Clearing "since" a time removes only logs uploaded after it.
        let old = logs_dir(&state(dir.path(), 1024).cfg).join("a-1.log");
        fs::File::options().write(true).open(&old).unwrap()
            .set_modified(SystemTime::now() - Duration::from_secs(7200)).unwrap();
        app.clone().oneshot(log_request("PUT", "b.log", Body::from("x"))).await.unwrap();
        let hour_ago = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() - 3_600_000;
        let clear_hour = axum::http::Request::builder().method("DELETE").uri(format!("/v1/logs?since={hour_ago}"))
            .header(header::AUTHORIZATION, format!("Bearer {TOKEN}")).body(Body::empty()).unwrap();
        let response = app.clone().oneshot(clear_hour).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json_body(response).await["removed"], 1);
        assert!(old.exists() && !logs_dir(&state(dir.path(), 1024).cfg).join("b.log").exists());
        let clear_all = axum::http::Request::builder().method("DELETE").uri("/v1/logs")
            .header(header::AUTHORIZATION, format!("Bearer {TOKEN}")).body(Body::empty()).unwrap();
        assert_eq!(json_body(app.oneshot(clear_all).await.unwrap()).await["removed"], 1);
        assert!(!old.exists());
    }

    #[test]
    fn session_logs_expire_after_the_retention_period() {
        let dir = TempDir::new().unwrap();
        let cfg = state(dir.path(), 1024).cfg.clone();
        fs::create_dir_all(logs_dir(&cfg)).unwrap();
        let now = SystemTime::now();
        for (name, age) in [("old.log", cfg.log_retention + Duration::from_secs(60)), ("fresh.log", Duration::from_secs(60))] {
            let path = logs_dir(&cfg).join(name);
            fs::write(&path, b"x").unwrap();
            fs::File::options().write(true).open(&path).unwrap().set_modified(now - age).unwrap();
        }
        assert_eq!(prune_logs(&cfg, now), 1);
        assert!(!logs_dir(&cfg).join("old.log").exists());
        assert!(logs_dir(&cfg).join("fresh.log").exists());
    }

    #[test]
    fn token_validation_rejects_short_and_placeholder_values() {
        assert!(validate_token("short").is_err());
        assert!(validate_token("replace-with-at-least-32-random-bytes").is_err());
        assert!(validate_token(TOKEN).is_ok());
    }
}
