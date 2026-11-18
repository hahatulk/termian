//! Native services the Electron main process used to provide: the OS
//! credential store (@termius/keytar), opening external links, and the
//! self-hosted sync ("self-cloud") configuration, encryption and transport.

use crate::termian_data_dir;
use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::PathBuf,
    sync::Mutex,
    time::Duration,
};

// -------------------------------------------------------------- keychain --

/// keytar exposes `findCredentials`, which OS stores cannot enumerate through
/// the keyring crate; keep a non-secret index of account names per service.
static KEYCHAIN_INDEX: Mutex<()> = Mutex::new(());

fn keychain_index_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(termian_data_dir(app)?.join("keychain-index.json"))
}

fn read_keychain_index(app: &tauri::AppHandle) -> BTreeMap<String, Vec<String>> {
    keychain_index_path(app)
        .ok()
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_keychain_index(app: &tauri::AppHandle, index: &BTreeMap<String, Vec<String>>) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(index).map_err(|error| error.to_string())?;
    fs::write(keychain_index_path(app)?, bytes).map_err(|error| error.to_string())
}

fn keychain_entry(service: &str, account: &str) -> Result<keyring::Entry, String> {
    if service.is_empty() || account.is_empty() || service.len() > 256 || account.len() > 256 {
        return Err("Invalid keychain entry name".into());
    }
    keyring::Entry::new(service, account).map_err(|error| error.to_string())
}

// Security.framework may display a macOS authorization prompt. Never block
// Tauri's main thread while waiting for the OS credential store.
#[tauri::command(async)]
pub fn keychain_get(service: String, account: String) -> Result<Option<String>, String> {
    match keychain_entry(&service, &account)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command(async)]
pub fn keychain_set(app: tauri::AppHandle, service: String, account: String, password: String) -> Result<(), String> {
    let _guard = KEYCHAIN_INDEX.lock().map_err(|_| "Keychain is busy")?;
    keychain_entry(&service, &account)?
        .set_password(&password)
        .map_err(|error| error.to_string())?;
    let mut index = read_keychain_index(&app);
    let accounts = index.entry(service).or_default();
    if !accounts.contains(&account) {
        accounts.push(account);
    }
    write_keychain_index(&app, &index)
}

#[tauri::command(async)]
pub fn keychain_delete(app: tauri::AppHandle, service: String, account: String) -> Result<bool, String> {
    let _guard = KEYCHAIN_INDEX.lock().map_err(|_| "Keychain is busy")?;
    let deleted = match keychain_entry(&service, &account)?.delete_credential() {
        Ok(()) => true,
        Err(keyring::Error::NoEntry) => false,
        Err(error) => return Err(error.to_string()),
    };
    let mut index = read_keychain_index(&app);
    if let Some(accounts) = index.get_mut(&service) {
        accounts.retain(|name| name != &account);
    }
    write_keychain_index(&app, &index)?;
    Ok(deleted)
}

#[tauri::command(async)]
pub fn keychain_find(app: tauri::AppHandle, service: String) -> Result<Vec<Value>, String> {
    let accounts = read_keychain_index(&app).remove(&service).unwrap_or_default();
    let mut found = Vec::new();
    for account in accounts {
        if let Some(password) = keychain_get(service.clone(), account.clone())? {
            found.push(json!({"account": account, "password": password}));
        }
    }
    Ok(found)
}

/// The menu's Exit item: also ends the hidden background window.
#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

// ---------------------------------------------------------- open external --

#[tauri::command]
pub fn open_external(url: String) -> Result<(), String> {
    let parsed = url::Url::parse(&url).map_err(|_| "Invalid link")?;
    // The file-access guide opens only the Privacy & Security panes.
    let privacy_pane = parsed.scheme() == "x-apple.systempreferences"
        && url.starts_with("x-apple.systempreferences:com.apple.preference.security?Privacy_")
        && url.len() < 120;
    if !matches!(parsed.scheme(), "https" | "http" | "mailto") && !privacy_pane {
        return Err("Only web and mail links can be opened".into());
    }
    tauri_plugin_opener::open_url(parsed.as_str(), None::<&str>).map_err(|error| error.to_string())
}

// ------------------------------------------------------------- self-cloud --

pub(crate) const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_PLAINTEXT_BYTES: usize = 47 * 1024 * 1024;
const MAX_SECRET_LENGTH: usize = 4096;
pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const ENVELOPE_VERSION: u32 = 1;
const WRONG_PASSPHRASE: &str = "Не удалось расшифровать снимок: неверный пароль шифрования";

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct SelfCloudConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    passphrase: Option<String>,
    /// HTTP Basic credentials for a reverse proxy in front of the sync server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    basic_user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    basic_password: Option<String>,
    /// Hourly two-way merge sync (src/bridge/self-cloud.js); on unless disabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    auto_sync: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfCloudChanges {
    url: String,
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    passphrase: Option<String>,
    /// `Some("")` turns Basic auth off; `None` keeps the saved credentials.
    #[serde(default, rename = "basicUser")]
    basic_user: Option<String>,
    #[serde(default, rename = "basicPassword")]
    basic_password: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Envelope {
    version: u32,
    salt: String,
    iv: String,
    tag: String,
    ciphertext: String,
}

fn self_cloud_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(termian_data_dir(app)?.join("self-cloud.json"))
}

pub(crate) fn read_self_cloud(app: &tauri::AppHandle) -> SelfCloudConfig {
    self_cloud_path(app)
        .ok()
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn validate_url(input: &str) -> Result<url::Url, String> {
    if input.is_empty() || input.len() > 2048 {
        return Err("Недопустимый адрес сервера".into());
    }
    let url = url::Url::parse(input).map_err(|_| "Недопустимый адрес сервера")?;
    if !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() || input.contains(['?', '#', '@']) {
        return Err("Адрес не должен содержать учетные данные, запрос или фрагмент".into());
    }
    if url.scheme() != "https" && !(url.scheme() == "http" && is_local_network_host(&url)) {
        return Err("Используйте HTTPS (HTTP разрешен только для локальной сети)".into());
    }
    Ok(url)
}

/// Plain HTTP sends the bearer token unencrypted, so it is accepted only for
/// a trusted local network: loopback, private/link-local IPv4 and IPv6,
/// CGNAT/Tailscale (100.64.0.0/10) and LAN host names. Snapshots themselves
/// are always sealed with the client-side passphrase.
fn is_local_network_host(url: &url::Url) -> bool {
    match url.host() {
        Some(url::Host::Ipv4(ip)) => {
            let [a, b, ..] = ip.octets();
            ip.is_loopback() || ip.is_private() || ip.is_link_local() || (a == 100 && (64..128).contains(&b))
        }
        Some(url::Host::Ipv6(ip)) => {
            let first = ip.segments()[0];
            ip.is_loopback() || (first & 0xfe00) == 0xfc00 || (first & 0xffc0) == 0xfe80
        }
        Some(url::Host::Domain(name)) => {
            let name = name.trim_end_matches('.').to_ascii_lowercase();
            !name.contains('.')
                || [".local", ".lan", ".home.arpa", ".internal", ".localhost"].iter().any(|suffix| name.ends_with(suffix))
        }
        None => false,
    }
}

fn validate_token(token: &str) -> Result<(), String> {
    if token.is_empty() || token.len() > MAX_SECRET_LENGTH || !token.bytes().all(|byte| (0x21..=0x7e).contains(&byte)) {
        return Err("Недопустимый токен".into());
    }
    Ok(())
}

fn validate_passphrase(passphrase: &str) -> Result<(), String> {
    if passphrase.chars().count() < 12 || passphrase.len() > MAX_SECRET_LENGTH {
        return Err("Пароль шифрования должен содержать не менее 12 символов".into());
    }
    Ok(())
}

fn validate_basic(user: &str, password: &str) -> Result<(), String> {
    let printable = |value: &str| value.chars().all(|ch| !ch.is_control());
    if user.is_empty() || user.contains(':') || user.len() > MAX_SECRET_LENGTH || !printable(user) {
        return Err("Недопустимый логин Basic auth".into());
    }
    if password.is_empty() || password.len() > MAX_SECRET_LENGTH || !printable(password) {
        return Err("Недопустимый пароль Basic auth".into());
    }
    Ok(())
}

pub(crate) struct ValidConfig {
    pub(crate) url: url::Url,
    pub(crate) token: String,
    pub(crate) passphrase: String,
    basic: Option<String>,
}

impl ValidConfig {
    /// With Basic auth the proxy owns `Authorization`, so the sync token moves
    /// to `X-Termian-Token` (accepted by cloud-sync-server).
    pub(crate) fn authorize(&self, request: ureq::Request) -> ureq::Request {
        match &self.basic {
            Some(basic) => request.set("authorization", &format!("Basic {basic}")).set("x-termian-token", &self.token),
            None => request.set("authorization", &format!("Bearer {}", self.token)),
        }
    }
}

pub(crate) fn validated(config: &SelfCloudConfig) -> Result<ValidConfig, String> {
    let url = validate_url(config.url.as_deref().unwrap_or(""))?;
    let token = config.token.clone().unwrap_or_default();
    validate_token(&token)?;
    let passphrase = config.passphrase.clone().unwrap_or_default();
    validate_passphrase(&passphrase)?;
    let basic = match config.basic_user.as_deref().filter(|user| !user.is_empty()) {
        Some(user) => {
            let password = config.basic_password.as_deref().unwrap_or("");
            validate_basic(user, password)?;
            Some(STANDARD.encode(format!("{user}:{password}")))
        }
        None => None,
    };
    Ok(ValidConfig { url, token, passphrase, basic })
}

fn metadata(config: &SelfCloudConfig) -> Value {
    let url = config.url.as_deref().filter(|url| validate_url(url).is_ok()).unwrap_or("");
    json!({
        "url": url,
        "hasToken": config.token.as_deref().is_some_and(|token| !token.is_empty()),
        "hasPassphrase": config.passphrase.as_deref().is_some_and(|passphrase| !passphrase.is_empty()),
        "basicUser": config.basic_user.as_deref().unwrap_or(""),
        "hasBasicPassword": config.basic_password.as_deref().is_some_and(|password| !password.is_empty()),
        "configured": validated(config).is_ok(),
        "autoSync": config.auto_sync != Some(false),
    })
}

/// Errors may echo server text; never let a secret reach the renderer.
pub(crate) fn redact(message: impl Into<String>, config: &SelfCloudConfig) -> String {
    let mut message: String = message.into();
    for secret in [config.token.as_deref(), config.passphrase.as_deref(), config.basic_password.as_deref()].into_iter().flatten() {
        if !secret.is_empty() {
            message = message.replace(secret, "[скрыто]");
        }
    }
    message.chars().take(240).collect()
}

#[tauri::command]
pub fn self_cloud_get_config(app: tauri::AppHandle) -> Value {
    metadata(&read_self_cloud(&app))
}

#[tauri::command]
pub fn self_cloud_save_config(app: tauri::AppHandle, config: SelfCloudChanges) -> Result<Value, String> {
    let previous = read_self_cloud(&app);
    let keep = |value: Option<String>, old: Option<String>| match value {
        Some(value) if !value.trim().is_empty() => Some(value),
        _ => old,
    };
    let next = SelfCloudConfig {
        url: Some(config.url),
        token: keep(config.token, previous.token.clone()),
        passphrase: keep(config.passphrase, previous.passphrase.clone()),
        basic_user: match config.basic_user {
            Some(user) => Some(user.trim().to_string()).filter(|user| !user.is_empty()),
            None => previous.basic_user.clone(),
        },
        basic_password: None,
        auto_sync: previous.auto_sync,
    };
    let mut next = next;
    next.basic_password = match next.basic_user {
        Some(_) => keep(config.basic_password, previous.basic_password.clone()),
        None => None,
    };
    validated(&next).map_err(|error| redact(error, &next))?;
    write_self_cloud(&app, &next)?;
    Ok(metadata(&next))
}

#[tauri::command]
pub fn self_cloud_set_auto_sync(app: tauri::AppHandle, enabled: bool) -> Result<Value, String> {
    let mut config = read_self_cloud(&app);
    config.auto_sync = Some(enabled);
    write_self_cloud(&app, &config)?;
    Ok(metadata(&config))
}

fn write_self_cloud(app: &tauri::AppHandle, next: &SelfCloudConfig) -> Result<(), String> {
    let path = self_cloud_path(app)?;
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(next).map_err(|error| error.to_string())?;
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    fs::rename(&temporary, &path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        error.to_string()
    })
}

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let params = scrypt::Params::new(14, 8, 1, 32).map_err(|error| error.to_string())?;
    let mut key = [0u8; 32];
    scrypt::scrypt(passphrase.as_bytes(), salt, &params, &mut key).map_err(|error| error.to_string())?;
    Ok(key)
}

/// Same envelope as the Electron self-cloud transport (scrypt N=16384, r=8,
/// p=1 → AES-256-GCM), so snapshots stay compatible with that client.
pub(crate) fn seal(snapshot: &Value, passphrase: &str) -> Result<Envelope, String> {
    if !snapshot.is_object() {
        return Err("snapshot must be a plain object".into());
    }
    let plaintext = serde_json::to_vec(snapshot).map_err(|error| error.to_string())?;
    if plaintext.len() > MAX_PLAINTEXT_BYTES {
        return Err("snapshot exceeds 47 MiB".into());
    }
    let mut salt = [0u8; 16];
    let mut iv = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut iv);
    let key = derive_key(passphrase, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|error| error.to_string())?;
    let mut sealed = cipher
        .encrypt(Nonce::from_slice(&iv), plaintext.as_slice())
        .map_err(|_| "encryption failed")?;
    let tag = sealed.split_off(sealed.len() - 16);
    Ok(Envelope {
        version: ENVELOPE_VERSION,
        salt: STANDARD.encode(salt),
        iv: STANDARD.encode(iv),
        tag: STANDARD.encode(tag),
        ciphertext: STANDARD.encode(sealed),
    })
}

fn decode_field(value: &str, name: &str, size: Option<usize>) -> Result<Vec<u8>, String> {
    let bytes = STANDARD.decode(value).map_err(|_| format!("invalid {name} base64"))?;
    if size.is_some_and(|size| bytes.len() != size) {
        return Err(format!("invalid {name} size"));
    }
    Ok(bytes)
}

pub(crate) fn open(envelope: &[u8], passphrase: &str) -> Result<Value, String> {
    let envelope: Envelope = serde_json::from_slice(envelope).map_err(|_| "invalid encryption envelope")?;
    if envelope.version != ENVELOPE_VERSION {
        return Err("invalid encryption envelope".into());
    }
    let salt = decode_field(&envelope.salt, "salt", Some(16))?;
    let iv = decode_field(&envelope.iv, "iv", Some(12))?;
    let tag = decode_field(&envelope.tag, "tag", Some(16))?;
    let mut ciphertext = decode_field(&envelope.ciphertext, "ciphertext", None)?;
    if ciphertext.len() > MAX_BYTES {
        return Err("ciphertext exceeds 64 MiB".into());
    }
    ciphertext.extend_from_slice(&tag);
    let key = derive_key(passphrase, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|error| error.to_string())?;
    // AES-GCM authentication failure: a wrong passphrase (or a tampered file).
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&iv), ciphertext.as_slice())
        .map_err(|_| WRONG_PASSPHRASE)?;
    let snapshot: Value = serde_json::from_slice(&plaintext).map_err(|_| "unable to decrypt snapshot")?;
    if !snapshot.is_object() {
        return Err("decrypted snapshot must be a plain object".into());
    }
    Ok(snapshot)
}

pub(crate) fn snapshot_endpoint(url: &url::Url) -> url::Url {
    let mut endpoint = url.clone();
    let mut path = url.path().to_string();
    while path.contains("//") {
        path = path.replace("//", "/");
    }
    let path = path.trim_end_matches('/');
    let path = if path.ends_with("/v1/snapshot") { path.to_string() } else { format!("{path}/v1/snapshot") };
    endpoint.set_path(&path);
    endpoint
}

/// Human-readable transport errors. 429 comes from the sync server's
/// failed-authorization limit and carries `Retry-After` in seconds.
pub(crate) fn status_error(context: &str, code: u16, response: &ureq::Response) -> String {
    match code {
        401 | 403 => format!("{context}: сервер отклонил авторизацию (HTTP {code}). Проверьте токен и логин/пароль Basic auth в настройках своего облака"),
        429 => {
            let wait = response
                .header("retry-after")
                .and_then(|value| value.trim().parse::<u64>().ok())
                .map(|seconds| format!(" Повторите через {}", human_wait(seconds)))
                .unwrap_or_default();
            format!("{context}: слишком много неверных попыток авторизации, сервер временно заблокировал доступ.{wait}")
        }
        _ => format!("{context}: HTTP {code}"),
    }
}

fn human_wait(seconds: u64) -> String {
    if seconds < 90 { format!("{seconds} с") } else { format!("{} мин", seconds.div_ceil(60)) }
}

// ------------------------------------------------ wrong passphrase limit --

/// The server only stores ciphertext and cannot see a wrong passphrase, so
/// failed decryptions are limited here. A few typos are free; afterwards
/// each failure doubles a lockout (30 s … 15 min). Persisted in the profile
/// so restarting the app does not reset it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
struct DecryptGuard {
    failures: u32,
    locked_until: u64,
}

const DECRYPT_FREE_FAILURES: u32 = 3;
const DECRYPT_BASE_LOCKOUT: u64 = 30;
const DECRYPT_MAX_LOCKOUT: u64 = 15 * 60;
static DECRYPT_GUARD: Mutex<()> = Mutex::new(());

impl DecryptGuard {
    fn remaining(&self, now: u64) -> Option<u64> {
        (self.locked_until > now).then(|| self.locked_until - now)
    }

    fn fail(&mut self, now: u64) -> Option<u64> {
        self.failures = self.failures.saturating_add(1);
        if self.failures <= DECRYPT_FREE_FAILURES {
            return None;
        }
        let exponent = (self.failures - DECRYPT_FREE_FAILURES - 1).min(10);
        let lockout = (DECRYPT_BASE_LOCKOUT << exponent).min(DECRYPT_MAX_LOCKOUT);
        self.locked_until = now + lockout;
        Some(lockout)
    }
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

fn decrypt_guard_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(termian_data_dir(app)?.join("self-cloud-decrypt-guard.json"))
}

fn read_decrypt_guard(app: &tauri::AppHandle) -> DecryptGuard {
    decrypt_guard_path(app)
        .ok()
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_decrypt_guard(app: &tauri::AppHandle, guard: &DecryptGuard) {
    if let (Ok(path), Ok(bytes)) = (decrypt_guard_path(app), serde_json::to_vec(guard)) {
        let _ = fs::write(path, bytes);
    }
}

fn locked_message(seconds: u64) -> String {
    format!("Слишком много неверных паролей шифрования. Повторите через {}", human_wait(seconds))
}

pub(crate) const REVISION_CHANGED: &str = "Снимок в облаке изменился во время синхронизации";

/// One snapshot request. `condition` is an If-Match / If-None-Match header:
/// a merged upload must not overwrite a snapshot another device sent after
/// it was downloaded. Returns the body and the snapshot revision (ETag).
fn http(method: &str, config: &ValidConfig, body: Option<Vec<u8>>, condition: Option<(&str, &str)>) -> Result<(Vec<u8>, Option<String>), String> {
    let request = ureq::request(method, snapshot_endpoint(&config.url).as_str())
        .timeout(REQUEST_TIMEOUT);
    let mut request = config.authorize(request);
    if let Some((name, value)) = condition {
        request = request.set(name, value);
    }
    let response = match body {
        Some(body) => request.set("content-type", "application/octet-stream").send_bytes(&body),
        None => request.call(),
    };
    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(412, _)) => return Err(format!("{REVISION_CHANGED} (HTTP 412)")),
        Err(ureq::Error::Status(code, response)) => {
            let action = if method == "PUT" { "Отправка в облако" } else { "Загрузка из облака" };
            return Err(status_error(action, code, &response));
        }
        Err(error) => return Err(error.to_string()),
    };
    let revision = response.header("etag").map(str::to_owned);
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > MAX_BYTES {
        return Err("response exceeds 64 MiB".into());
    }
    Ok((bytes, revision))
}

/// Check if an encrypted snapshot exists without downloading it. The opaque
/// ciphertext ETag never contains decrypted vault data.
#[tauri::command]
pub async fn self_cloud_probe(app: tauri::AppHandle) -> Result<Value, String> {
    let stored = read_self_cloud(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let config = validated(&stored)?;
        let response = config.authorize(ureq::request("HEAD", snapshot_endpoint(&config.url).as_str()).timeout(REQUEST_TIMEOUT)).call();
        match response {
            Ok(reply) => Ok(json!({"exists": true, "revision": reply.header("etag")})),
            Err(ureq::Error::Status(404, _)) => Ok(json!({"exists": false, "revision": null})),
            Err(ureq::Error::Status(code, response)) => Err(status_error("Проверка облака", code, &response)),
            Err(error) => Err(error.to_string()),
        }
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| redact(error, &read_self_cloud(&app)))
}

/// `revision`: replace only that snapshot revision; `create`: only if none
/// exists yet. Without either the snapshot is replaced unconditionally.
#[tauri::command]
pub async fn self_cloud_upload(app: tauri::AppHandle, snapshot: Value, revision: Option<String>, create: Option<bool>) -> Result<Value, String> {
    let stored = read_self_cloud(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let config = validated(&stored)?;
        let envelope = seal(&snapshot, &config.passphrase)?;
        let body = serde_json::to_vec(&envelope).map_err(|error| error.to_string())?;
        let condition = match (&revision, create) {
            (Some(revision), _) => Some(("if-match", revision.as_str())),
            (None, Some(true)) => Some(("if-none-match", "*")),
            _ => None,
        };
        let (_, revision) = http("PUT", &config, Some(body), condition)?;
        Ok::<_, String>(json!({"revision": revision}))
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| redact(error, &read_self_cloud(&app)))
}

/// Decrypt a downloaded snapshot under the wrong-passphrase limit.
fn open_guarded(app: &tauri::AppHandle, body: &[u8], passphrase: &str) -> Result<Value, String> {
    let result = open(body, passphrase);
    let _lock = DECRYPT_GUARD.lock().map_err(|_| "Проверка пароля занята")?;
    let mut guard = read_decrypt_guard(app);
    match &result {
        Err(error) if error == WRONG_PASSPHRASE => {
            let lockout = guard.fail(unix_now());
            write_decrypt_guard(app, &guard);
            return Err(match lockout {
                Some(seconds) => format!("{WRONG_PASSPHRASE}. {}", locked_message(seconds)),
                None => format!("{WRONG_PASSPHRASE}. Попыток без блокировки осталось: {}",
                    DECRYPT_FREE_FAILURES.saturating_sub(guard.failures)),
            });
        }
        Ok(_) if guard != DecryptGuard::default() => write_decrypt_guard(app, &DecryptGuard::default()),
        _ => {}
    }
    result
}

#[tauri::command]
pub async fn self_cloud_download(app: tauri::AppHandle) -> Result<Value, String> {
    let stored = read_self_cloud(&app);
    let guard_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let config = validated(&stored)?;
        if let Some(left) = read_decrypt_guard(&guard_app).remaining(unix_now()) {
            return Err(locked_message(left));
        }
        let (body, _) = http("GET", &config, None, None)?;
        open_guarded(&guard_app, &body, &config.passphrase)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| redact(error, &read_self_cloud(&app)))
}

/// Snapshot with its revision for a merge: `{exists, revision, snapshot}`.
#[tauri::command]
pub async fn self_cloud_fetch(app: tauri::AppHandle) -> Result<Value, String> {
    let stored = read_self_cloud(&app);
    let guard_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let config = validated(&stored)?;
        if let Some(left) = read_decrypt_guard(&guard_app).remaining(unix_now()) {
            return Err(locked_message(left));
        }
        let (body, revision) = match http("GET", &config, None, None) {
            Ok(reply) => reply,
            Err(error) if error.ends_with("HTTP 404") => return Ok(json!({"exists": false, "revision": null, "snapshot": null})),
            Err(error) => return Err(error),
        };
        let snapshot = open_guarded(&guard_app, &body, &config.passphrase)?;
        Ok(json!({"exists": true, "revision": revision, "snapshot": snapshot}))
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| redact(error, &read_self_cloud(&app)))
}

/// Encrypted local backup taken before a download replaces the vault.
#[tauri::command]
pub async fn self_cloud_backup(app: tauri::AppHandle, snapshot: Value) -> Result<(), String> {
    let stored = read_self_cloud(&app);
    let dir = termian_data_dir(&app)?.join("self-cloud-backups");
    tauri::async_runtime::spawn_blocking(move || {
        let config = validated(&stored)?;
        fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
        let envelope = seal(&snapshot, &config.passphrase)?;
        let stamp = chrono_like_stamp();
        fs::write(dir.join(format!("{stamp}.json")), serde_json::to_vec(&envelope).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
        let mut backups: Vec<_> = fs::read_dir(&dir)
            .map_err(|error| error.to_string())?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
            .collect();
        backups.sort();
        backups.reverse();
        for old in backups.into_iter().skip(10) {
            let _ = fs::remove_file(old);
        }
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error: String| redact(error, &read_self_cloud(&app)))
}

fn chrono_like_stamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:013}", now.as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_auth_moves_token_to_custom_header() {
        let mut config = SelfCloudConfig {
            url: Some("https://cloud.example".into()),
            token: Some("sync-token".into()),
            passphrase: Some("correct horse battery".into()),
            ..Default::default()
        };
        let plain = validated(&config).unwrap().authorize(ureq::get("https://cloud.example"));
        assert_eq!(plain.header("authorization"), Some("Bearer sync-token"));
        assert_eq!(plain.header("x-termian-token"), None);

        config.basic_user = Some("alice".into());
        assert!(validated(&config).is_err(), "Basic auth needs a password");
        config.basic_password = Some("s3cret".into());
        let proxied = validated(&config).unwrap().authorize(ureq::get("https://cloud.example"));
        assert_eq!(proxied.header("authorization"), Some("Basic YWxpY2U6czNjcmV0"));
        assert_eq!(proxied.header("x-termian-token"), Some("sync-token"));
        assert!(!redact("bad s3cret", &config).contains("s3cret"));
        assert_eq!(metadata(&config)["hasBasicPassword"], true);

        config.basic_user = Some("a:b".into());
        assert!(validated(&config).is_err());
    }

    #[test]
    fn sealed_snapshots_round_trip_and_reject_wrong_passphrase() {
        let snapshot = json!({"format": "termius-portable-snapshot", "hosts": [1, 2]});
        let envelope = seal(&snapshot, "correct horse battery").expect("seal");
        let bytes = serde_json::to_vec(&envelope).expect("encode");
        assert_eq!(open(&bytes, "correct horse battery").expect("open"), snapshot);
        assert_eq!(open(&bytes, "wrong passphrase!!").unwrap_err(), WRONG_PASSPHRASE);
    }

    #[test]
    fn wrong_passphrases_are_free_three_times_then_lock_out_exponentially() {
        let mut guard = DecryptGuard::default();
        for _ in 0..DECRYPT_FREE_FAILURES {
            assert_eq!(guard.fail(1000), None);
        }
        assert_eq!(guard.remaining(1000), None);
        assert_eq!(guard.fail(1000), Some(30));
        assert_eq!(guard.remaining(1010), Some(20));
        assert_eq!(guard.remaining(1030), None);
        assert_eq!(guard.fail(2000), Some(60));
        assert_eq!(guard.fail(3000), Some(120));
        for _ in 0..20 { guard.fail(4000); }
        assert_eq!(guard.fail(5000), Some(DECRYPT_MAX_LOCKOUT));
        assert_eq!(human_wait(45), "45 с");
        assert_eq!(human_wait(600), "10 мин");
    }

    #[test]
    fn endpoints_and_urls_are_validated() {
        let url = validate_url("https://cloud.example/api/").expect("valid");
        assert_eq!(snapshot_endpoint(&url).as_str(), "https://cloud.example/api/v1/snapshot");
        assert!(validate_url("http://cloud.example").is_err());
        assert!(validate_url("http://127.0.0.1:8080").is_ok());
        // Trusted local network: private IPv4/IPv6, Tailscale/CGNAT, LAN names.
        for lan in ["http://192.168.1.10:8080", "http://10.0.0.5", "http://172.16.3.4:8080",
                    "http://172.31.255.1", "http://169.254.10.1", "http://100.100.1.2",
                    "http://[fd00::5]:8080", "http://[fe80::1]", "http://nas:8080",
                    "http://nas.local", "http://sync.lan", "http://box.home.arpa", "http://srv.internal"] {
            assert!(validate_url(lan).is_ok(), "{lan}");
        }
        for public in ["http://8.8.8.8", "http://172.32.0.1", "http://100.128.0.1",
                       "http://[2001:db8::1]", "http://sync.example.com"] {
            assert!(validate_url(public).is_err(), "{public}");
        }
        assert!(validate_url("https://user:pw@cloud.example").is_err());
    }
}
