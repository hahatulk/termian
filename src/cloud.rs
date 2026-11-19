//! «Своё облако»: the user's own sync server (cloud-sync-server). The server
//! only stores ciphertext; snapshots are sealed here with the encryption
//! password, in the same envelope as the legacy client (legacy/src/rust/native.rs):
//! scrypt (N=16384, r=8, p=1) → AES-256-GCM, JSON `{version, salt, iv, tag, ciphertext}`.

use std::io::Read;
use std::time::Duration;

use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const ENVELOPE_VERSION: u32 = 1;
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_PLAINTEXT_BYTES: usize = 47 * 1024 * 1024;
const MAX_SECRET_LENGTH: usize = 4096;
const CONDITIONAL_CREATE: &str = "x-termian-conditional-create";
const LOG_GENERATION: &str = "x-termian-key-generation";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub const WRONG_PASSPHRASE: &str = "Не удалось расшифровать снимок: неверный пароль шифрования";
pub const REVISION_CHANGED: &str = "Снимок в облаке изменился во время синхронизации";

/// Saved inside the encrypted native vault, never in a plain file.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CloudConfig {
    pub url: String,
    pub token: String,
    pub passphrase: String,
    #[serde(default)]
    pub basic_user: String,
    #[serde(default)]
    pub basic_password: String,
    /// Hourly sync; on unless turned off.
    #[serde(default = "enabled")]
    pub auto_sync: bool,
}

fn enabled() -> bool {
    true
}

/// A config that passed the legacy checks, ready for requests.
#[derive(Clone, Debug)]
pub struct Cloud {
    endpoint: url::Url,
    token: String,
    passphrase: String,
    basic: Option<String>,
}

impl CloudConfig {
    pub fn validate(&self) -> Result<Cloud, String> {
        let url = validate_url(&self.url)?;
        if self.token.is_empty() || self.token.len() > MAX_SECRET_LENGTH || !self.token.bytes().all(|b| (0x21..=0x7e).contains(&b)) {
            return Err("Недопустимый токен".into());
        }
        if self.passphrase.chars().count() < 12 || self.passphrase.len() > MAX_SECRET_LENGTH {
            return Err("Пароль шифрования должен содержать не менее 12 символов".into());
        }
        let basic = if self.basic_user.is_empty() {
            None
        } else {
            let printable = |value: &str| value.chars().all(|c| !c.is_control());
            if self.basic_user.contains(':') || self.basic_user.len() > MAX_SECRET_LENGTH || !printable(&self.basic_user) {
                return Err("Недопустимый логин Basic auth".into());
            }
            if self.basic_password.is_empty() || self.basic_password.len() > MAX_SECRET_LENGTH || !printable(&self.basic_password) {
                return Err("Недопустимый пароль Basic auth".into());
            }
            Some(STANDARD.encode(format!("{}:{}", self.basic_user, self.basic_password)))
        };
        Ok(Cloud { endpoint: snapshot_endpoint(&url), token: self.token.clone(), passphrase: self.passphrase.clone(), basic })
    }

    /// Removes the secrets from a message that may echo server text.
    pub fn redact(&self, message: &str) -> String {
        let mut message = message.to_owned();
        for secret in [&self.token, &self.passphrase, &self.basic_password] {
            if !secret.is_empty() {
                message = message.replace(secret.as_str(), "***");
            }
        }
        message
    }
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

/// Plain HTTP is accepted only inside a trusted local network, as in the legacy client.
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
            !name.contains('.') || [".local", ".lan", ".home.arpa", ".internal", ".localhost"].iter().any(|s| name.ends_with(s))
        }
        None => false,
    }
}

/// `https://host/base` → `https://host/base/v1/snapshot`.
pub fn snapshot_endpoint(url: &url::Url) -> url::Url {
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

#[derive(Serialize, Deserialize)]
struct Envelope {
    version: u32,
    salt: String,
    iv: String,
    tag: String,
    ciphertext: String,
}

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let params = scrypt::Params::new(14, 8, 1, 32).map_err(|e| e.to_string())?;
    let mut key = [0u8; 32];
    scrypt::scrypt(passphrase.as_bytes(), salt, &params, &mut key).map_err(|e| e.to_string())?;
    Ok(key)
}

pub fn seal(snapshot: &Value, passphrase: &str) -> Result<Vec<u8>, String> {
    if !snapshot.is_object() {
        return Err("snapshot must be a plain object".into());
    }
    let plaintext = serde_json::to_vec(snapshot).map_err(|e| e.to_string())?;
    if plaintext.len() > MAX_PLAINTEXT_BYTES {
        return Err("snapshot exceeds 47 MiB".into());
    }
    let (mut salt, mut iv) = ([0u8; 16], [0u8; 12]);
    getrandom::getrandom(&mut salt).map_err(|e| e.to_string())?;
    getrandom::getrandom(&mut iv).map_err(|e| e.to_string())?;
    let cipher = Aes256Gcm::new_from_slice(&derive_key(passphrase, &salt)?).map_err(|e| e.to_string())?;
    let mut sealed = cipher.encrypt(Nonce::from_slice(&iv), plaintext.as_slice()).map_err(|_| "encryption failed")?;
    let tag = sealed.split_off(sealed.len() - 16);
    let envelope = Envelope { version: ENVELOPE_VERSION, salt: STANDARD.encode(salt), iv: STANDARD.encode(iv), tag: STANDARD.encode(tag), ciphertext: STANDARD.encode(sealed) };
    serde_json::to_vec(&envelope).map_err(|e| e.to_string())
}

pub fn open(envelope: &[u8], passphrase: &str) -> Result<Value, String> {
    let envelope: Envelope = serde_json::from_slice(envelope).map_err(|_| "invalid encryption envelope")?;
    if envelope.version != ENVELOPE_VERSION {
        return Err("invalid encryption envelope".into());
    }
    let field = |value: &str, name: &str, size: Option<usize>| -> Result<Vec<u8>, String> {
        let bytes = STANDARD.decode(value).map_err(|_| format!("invalid {name} base64"))?;
        if size.is_some_and(|size| bytes.len() != size) {
            return Err(format!("invalid {name} size"));
        }
        Ok(bytes)
    };
    let salt = field(&envelope.salt, "salt", Some(16))?;
    let iv = field(&envelope.iv, "iv", Some(12))?;
    let tag = field(&envelope.tag, "tag", Some(16))?;
    let mut ciphertext = field(&envelope.ciphertext, "ciphertext", None)?;
    if ciphertext.len() > MAX_BYTES {
        return Err("ciphertext exceeds 64 MiB".into());
    }
    ciphertext.extend_from_slice(&tag);
    let cipher = Aes256Gcm::new_from_slice(&derive_key(passphrase, &salt)?).map_err(|e| e.to_string())?;
    let plaintext = cipher.decrypt(Nonce::from_slice(&iv), ciphertext.as_slice()).map_err(|_| WRONG_PASSPHRASE)?;
    let snapshot: Value = serde_json::from_slice(&plaintext).map_err(|_| "unable to decrypt snapshot")?;
    if !snapshot.is_object() {
        return Err("decrypted snapshot must be a plain object".into());
    }
    Ok(snapshot)
}

/// A snapshot on the server with its revision (`ETag`).
#[derive(Clone, Debug, PartialEq)]
pub struct Remote {
    pub snapshot: Value,
    pub revision: Option<String>,
}

/// The cloud's answer to a log cleanup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogsRemoved {
    Removed(u64),
    /// An older server without the logs API.
    Unsupported,
}

/// How an upload must relate to what is on the server.
#[derive(Clone, Debug, PartialEq)]
pub enum Condition {
    /// Replace whatever is there ("Отправить").
    Any,
    /// Only over this revision (`If-Match`); otherwise [`REVISION_CHANGED`].
    Revision(String),
    /// Only if there is no snapshot yet (`If-None-Match: *`).
    Create,
}

fn status_error(context: &str, code: u16, retry_after: Option<&str>) -> String {
    match code {
        401 | 403 => format!("{context}: сервер отклонил авторизацию (HTTP {code}). Проверьте токен и логин/пароль Basic auth в настройках своего облака"),
        429 => {
            let wait = retry_after
                .and_then(|value| value.trim().parse::<u64>().ok())
                .map(|s| if s < 90 { format!(" Повторите через {s} с") } else { format!(" Повторите через {} мин", s.div_ceil(60)) })
                .unwrap_or_default();
            format!("{context}: слишком много неверных попыток авторизации, сервер временно заблокировал доступ.{wait}")
        }
        _ => format!("{context}: HTTP {code}"),
    }
}

fn strong_etag(response: &ureq::Response) -> Option<String> {
    let value = response.header("etag")?;
    let opaque = value.strip_prefix('"')?.strip_suffix('"')?;
    opaque.bytes().all(|b| b == b'!' || (b'#'..=b'~').contains(&b)).then(|| value.to_owned())
}

impl Cloud {
    fn request(&self, method: &str) -> ureq::Request {
        self.request_to(method, &self.endpoint)
    }

    fn request_to(&self, method: &str, endpoint: &url::Url) -> ureq::Request {
        let request = ureq::AgentBuilder::new().redirects(0).build()
            .request(method, endpoint.as_str()).timeout(REQUEST_TIMEOUT);
        // With Basic auth the proxy owns Authorization; the token moves to X-Termian-Token.
        match &self.basic {
            Some(basic) => request.set("authorization", &format!("Basic {basic}")).set("x-termian-token", &self.token),
            None => request.set("authorization", &format!("Bearer {}", self.token)),
        }
    }

    fn call(&self, method: &str, body: Option<Vec<u8>>, condition: &Condition) -> Result<ureq::Response, String> {
        let mut request = self.request(method);
        request = match condition {
            Condition::Any => request,
            Condition::Revision(revision) => request.set("if-match", revision),
            Condition::Create => request.set("if-none-match", "*"),
        };
        let response = match body {
            Some(body) => request.set("content-type", "application/octet-stream").send_bytes(&body),
            None => request.call(),
        };
        match response {
            Ok(response) if (200..300).contains(&response.status()) || (method != "PUT" && response.status() == 404) => Ok(response),
            Err(ureq::Error::Status(404, response)) if method != "PUT" => Ok(response),
            Err(ureq::Error::Status(412, _)) => Err(format!("{REVISION_CHANGED} (HTTP 412)")),
            other => {
                let response = match other {
                    Ok(response) | Err(ureq::Error::Status(_, response)) => response,
                    Err(error) => return Err(error.to_string()),
                };
                let context = if method == "PUT" { "Отправка в облако" } else { "Загрузка из облака" };
                Err(status_error(context, response.status(), response.header("retry-after")))
            }
        }
    }

    pub fn probe(&self) -> Result<Option<String>, String> {
        let response = self.call("HEAD", None, &Condition::Any)?;
        Ok((response.status() != 404).then(|| strong_etag(&response).unwrap_or_default()))
    }

    pub fn fetch(&self) -> Result<Option<Remote>, String> {
        self.fetch_with_headers().map(|(remote, _, _)| remote)
    }

    pub(crate) fn fetch_for_sync(&self) -> Result<(Option<Remote>, bool), String> {
        self.fetch_with_headers().map(|(remote, can_create, _)| (remote, can_create))
    }

    fn fetch_with_headers(&self) -> Result<(Option<Remote>, bool, Option<String>), String> {
        let response = self.call("GET", None, &Condition::Any)?;
        let generation = response.header(LOG_GENERATION).map(str::to_owned);
        if response.status() == 404 {
            return Ok((None, response.header(CONDITIONAL_CREATE) == Some("v1"), generation));
        }
        let revision = strong_etag(&response);
        let mut body = Vec::new();
        response.into_reader().take(MAX_BYTES as u64 + 1).read_to_end(&mut body).map_err(|e| e.to_string())?;
        if body.len() > MAX_BYTES {
            return Err("response exceeds 64 MiB".into());
        }
        Ok((Some(Remote { snapshot: open(&body, &self.passphrase)?, revision }), false, generation))
    }

    pub(crate) fn log_generation(&self) -> Result<String, String> {
        self.fetch_with_headers()?.2.ok_or_else(|| "Сервер не поддерживает проверку пароля при отправке журналов. Обновите сервер.".into())
    }

    pub(crate) fn storage_key(&self) -> String {
        crate::sync::server_key(self.endpoint.as_str())
    }

    /// `DELETE /v1/logs?since=` (Unix ms, 0 = all): the cloud copies of session logs.
    pub fn delete_logs(&self, since: u64) -> Result<LogsRemoved, String> {
        let mut endpoint = self.endpoint.clone();
        let path = endpoint.path().trim_end_matches("/snapshot").to_owned();
        endpoint.set_path(&format!("{path}/logs"));
        endpoint.set_query(Some(&format!("since={since}")));
        match self.request_to("DELETE", &endpoint).call() {
            Ok(response) if (200..300).contains(&response.status()) => {
                let body: Value = response.into_string().ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
                Ok(LogsRemoved::Removed(body["removed"].as_u64().unwrap_or(0)))
            }
            Err(ureq::Error::Status(404 | 405, _)) => Ok(LogsRemoved::Unsupported),
            Ok(response) => Err(status_error("Очистка логов в облаке", response.status(), None)),
            Err(ureq::Error::Status(code, response)) => Err(status_error("Очистка логов в облаке", code, response.header("retry-after"))),
            Err(error) => Err(error.to_string()),
        }
    }

    /// `<server>/v1/logs/<name>` next to the snapshot endpoint.
    fn log_endpoint(&self, name: &str) -> url::Url {
        let mut endpoint = self.endpoint.clone();
        let path = endpoint.path().trim_end_matches("/snapshot").to_owned();
        endpoint.set_path(&format!("{path}/logs/{name}"));
        endpoint
    }

    /// Stores a session log backup sealed like a snapshot. `Ok(false)`: the
    /// server is older than the logs API (404/405).
    pub(crate) fn put_log(&self, name: &str, payload: &Value, generation: &str) -> Result<bool, String> {
        let body = seal(payload, &self.passphrase)?;
        let request = self.request_to("PUT", &self.log_endpoint(name)).set(LOG_GENERATION, generation);
        match request.set("content-type", "application/octet-stream").send_bytes(&body) {
            Ok(response) if (200..300).contains(&response.status()) => Ok(true),
            Err(ureq::Error::Status(404 | 405, _)) => Ok(false),
            Ok(response) => Err(status_error("Отправка лога сессии", response.status(), None)),
            Err(ureq::Error::Status(code, response)) => Err(status_error("Отправка лога сессии", code, response.header("retry-after"))),
            Err(error) => Err(error.to_string()),
        }
    }

    /// The decrypted log backup, or `None` when the cloud has none.
    pub fn get_log(&self, name: &str) -> Result<Option<Value>, String> {
        let response = match self.request_to("GET", &self.log_endpoint(name)).call() {
            Ok(response) if (200..300).contains(&response.status()) => response,
            Err(ureq::Error::Status(404 | 405, _)) => return Ok(None),
            Ok(response) => return Err(status_error("Загрузка лога сессии", response.status(), None)),
            Err(ureq::Error::Status(code, response)) => return Err(status_error("Загрузка лога сессии", code, response.header("retry-after"))),
            Err(error) => return Err(error.to_string()),
        };
        let mut body = Vec::new();
        response.into_reader().take(MAX_BYTES as u64 + 1).read_to_end(&mut body).map_err(|e| e.to_string())?;
        Ok(Some(open(&body, &self.passphrase)?))
    }

    /// Seals and stores a snapshot; returns the new revision.
    pub fn upload(&self, snapshot: &Value, condition: Condition) -> Result<Option<String>, String> {
        let body = seal(snapshot, &self.passphrase)?;
        Ok(strong_etag(&self.call("PUT", Some(body), &condition)?))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::sync::{Arc, Mutex};

    const PASSPHRASE: &str = "correct horse battery";

    pub(crate) fn config(url: &str) -> CloudConfig {
        CloudConfig { url: url.into(), token: "test-token".into(), passphrase: PASSPHRASE.into(), auto_sync: true, ..CloudConfig::default() }
    }

    #[test]
    fn opens_an_envelope_sealed_like_the_legacy_client() {
        // Made with Node crypto: scrypt N=16384 r=8 p=1, AES-256-GCM, salt 1s, iv 2s.
        let envelope = r#"{"version":1,"salt":"AQEBAQEBAQEBAQEBAQEBAQ==","iv":"AgICAgICAgICAgIC","tag":"NcUZW1pInEmOOyQorDLN+w==","ciphertext":"JNttguCmngG/U5e8cNgaqNdTdIBAz+twSdpe1mu0FUrtvfJ8Y8LsKipcaU4VsfuTF4PRATJ7UNzWsQwnHg=="}"#;
        let snapshot = open(envelope.as_bytes(), PASSPHRASE).unwrap();
        assert_eq!(snapshot["format"], "termius-portable-snapshot");
        assert_eq!(open(envelope.as_bytes(), "wrong password!!"), Err(WRONG_PASSPHRASE.into()));
    }

    #[test]
    fn seals_what_it_opens() {
        let snapshot = serde_json::json!({"format": "termius-portable-snapshot", "version": 1, "hosts": [{"label": "secret-host"}]});
        let sealed = seal(&snapshot, PASSPHRASE).unwrap();
        assert!(!String::from_utf8_lossy(&sealed).contains("secret-host"));
        assert_eq!(open(&sealed, PASSPHRASE).unwrap(), snapshot);
        assert!(seal(&serde_json::json!([1]), PASSPHRASE).is_err());
    }

    #[test]
    fn config_rules_match_the_legacy_settings() {
        assert!(config("https://sync.example.com").validate().is_ok());
        assert!(config("http://192.168.1.10:8080").validate().is_ok(), "HTTP inside the LAN");
        assert!(config("http://nas.local:8080").validate().is_ok());
        assert_eq!(config("http://sync.example.com").validate().unwrap_err(), "Используйте HTTPS (HTTP разрешен только для локальной сети)");
        assert!(config("https://user:pw@sync.example.com").validate().is_err());
        assert!(CloudConfig { passphrase: "short".into(), ..config("https://a.example") }.validate().is_err());
        assert!(CloudConfig { token: "has space".into(), ..config("https://a.example") }.validate().is_err());
        assert!(CloudConfig { basic_user: "u".into(), ..config("https://a.example") }.validate().is_err(), "Basic needs a password");
        let endpoint = |u: &str| snapshot_endpoint(&url::Url::parse(u).unwrap()).to_string();
        assert_eq!(endpoint("https://h.example/base//"), "https://h.example/base/v1/snapshot");
        assert_eq!(endpoint("https://h.example/v1/snapshot"), "https://h.example/v1/snapshot");
        let c = config("https://a.example");
        assert_eq!(c.redact("token test-token rejected"), "token *** rejected");
    }

    /// A tiny cloud-sync-server: one snapshot, ETag revisions, If-Match / If-None-Match, auth.
    /// Session log backups of every test server, keyed by server URL and name.
    static LOGS: std::sync::LazyLock<Mutex<std::collections::HashMap<String, Vec<u8>>>> = std::sync::LazyLock::new(Default::default);

    pub(crate) fn server() -> (String, Arc<Mutex<Option<(Vec<u8>, u64)>>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let stored: Arc<Mutex<Option<(Vec<u8>, u64)>>> = Arc::default();
        let state = stored.clone();
        let url_key = url.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() {
                    continue;
                }
                let method = line.split_whitespace().next().unwrap_or("").to_owned();
                let path = line.split_whitespace().nth(1).unwrap_or("").to_owned();
                let (mut length, mut auth, mut if_match, mut if_none) = (0usize, String::new(), None, None);
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    let header = header.trim_end();
                    if header.is_empty() {
                        break;
                    }
                    let (name, value) = header.split_once(':').unwrap();
                    match name.to_ascii_lowercase().as_str() {
                        "content-length" => length = value.trim().parse().unwrap(),
                        "authorization" => auth = value.trim().to_owned(),
                        "if-match" => if_match = Some(value.trim().to_owned()),
                        "if-none-match" => if_none = Some(value.trim().to_owned()),
                        _ => {}
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let mut snapshot = state.lock().unwrap();
                let etag = |rev: u64| format!("\"r{rev}\"");
                let (status, reply, tag) = if auth != "Bearer test-token" {
                    ("401 Unauthorized", Vec::new(), None)
                } else if let Some(name) = path.strip_prefix("/v1/logs/") {
                    let mut logs = LOGS.lock().unwrap();
                    let key = format!("{url_key}/{name}");
                    match method.as_str() {
                        "PUT" => {
                            logs.insert(key, body);
                            ("200 OK", Vec::new(), None)
                        }
                        _ => match logs.get(&key) {
                            Some(bytes) => ("200 OK", bytes.clone(), None),
                            None => ("404 Not Found", Vec::new(), None),
                        },
                    }
                } else if path.starts_with("/v1/logs") {
                    // cloud-sync-server answers a cleanup with how many logs it removed.
                    let removed = if method == "DELETE" && path == "/v1/logs?since=123" { 7 } else { 0 };
                    ("200 OK", format!("{{\"removed\":{removed}}}").into_bytes(), None)
                } else {
                    match method.as_str() {
                        "GET" | "HEAD" => match &*snapshot {
                            Some((bytes, rev)) => ("200 OK", if method == "GET" { bytes.clone() } else { Vec::new() }, Some(etag(*rev))),
                            None => ("404 Not Found", Vec::new(), None),
                        },
                        _ => {
                            let current = snapshot.as_ref().map(|(_, rev)| etag(*rev));
                            let refused = if_match.as_ref().is_some_and(|m| Some(m) != current.as_ref()) || (if_none.is_some() && current.is_some());
                            if refused {
                                ("412 Precondition Failed", Vec::new(), None)
                            } else {
                                let rev = snapshot.as_ref().map_or(1, |(_, rev)| rev + 1);
                                *snapshot = Some((body, rev));
                                ("200 OK", Vec::new(), Some(etag(rev)))
                            }
                        }
                    }
                };
                let mut out = stream;
                let tag = tag.map(|t| format!("ETag: {t}\r\n")).unwrap_or_default();
                let length = if method == "HEAD" { 0 } else { reply.len() };
                let _ = write!(out, "HTTP/1.1 {status}\r\n{tag}X-Termian-Conditional-Create: v1\r\nX-Termian-Key-Generation: initial\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n");
                let _ = out.write_all(&reply);
            }
        });
        (url, stored)
    }

    #[test]
    fn log_cleanup_reaches_the_logs_endpoint() {
        let (url, stored) = server();
        let cloud = config(&format!("{url}/v1/snapshot")).validate().unwrap();
        assert_eq!(cloud.delete_logs(123).unwrap(), LogsRemoved::Removed(7));
        assert_eq!(cloud.delete_logs(0).unwrap(), LogsRemoved::Removed(0));
        assert!(stored.lock().unwrap().is_none(), "the snapshot is untouched");
        let wrong = CloudConfig { token: "other".into(), ..config(&url) }.validate().unwrap();
        assert!(wrong.delete_logs(0).unwrap_err().contains("отклонил авторизацию"));
    }

    #[test]
    fn uploads_fetches_and_refuses_stale_writes() {
        let (url, _) = server();
        let cloud = config(&url).validate().unwrap();
        assert_eq!(cloud.fetch().unwrap(), None);
        assert_eq!(cloud.probe().unwrap(), None);
        let first = serde_json::json!({"hosts": [1]});
        let revision = cloud.upload(&first, Condition::Create).unwrap().unwrap();
        assert!(cloud.upload(&first, Condition::Create).unwrap_err().starts_with(REVISION_CHANGED), "create only once");
        let remote = cloud.fetch().unwrap().unwrap();
        assert_eq!((remote.snapshot.clone(), remote.revision.clone()), (first, Some(revision.clone())));
        assert_eq!(cloud.probe().unwrap(), Some(revision.clone()));

        let second = serde_json::json!({"hosts": [1, 2]});
        let newer = cloud.upload(&second, Condition::Revision(revision.clone())).unwrap().unwrap();
        // Another device still holding the old revision must not overwrite.
        assert!(cloud.upload(&serde_json::json!({}), Condition::Revision(revision)).unwrap_err().starts_with(REVISION_CHANGED));
        assert_eq!(cloud.fetch().unwrap().unwrap().snapshot, second);
        cloud.upload(&serde_json::json!({"replaced": true}), Condition::Any).unwrap();
        assert_ne!(cloud.probe().unwrap(), Some(newer));
    }

    #[test]
    fn a_wrong_token_or_password_is_reported() {
        let (url, _) = server();
        let wrong_token = CloudConfig { token: "other".into(), ..config(&url) }.validate().unwrap();
        assert!(wrong_token.fetch().unwrap_err().contains("сервер отклонил авторизацию (HTTP 401)"));
        let cloud = config(&url).validate().unwrap();
        cloud.upload(&serde_json::json!({}), Condition::Any).unwrap();
        let wrong_password = CloudConfig { passphrase: "another long passphrase".into(), ..config(&url) }.validate().unwrap();
        assert_eq!(wrong_password.fetch().unwrap_err(), WRONG_PASSPHRASE);
    }
}
