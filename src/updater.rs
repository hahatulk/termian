//! Automatic updates from GitHub Releases. Packages in `latest.json` carry
//! a minisign signature bound to the version in the signed comment.

use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const PUBLIC_KEY: &str = include_str!("../assets/app/update-public-key");
const REPOSITORY: &str = match option_env!("TERMIAN_UPDATE_REPOSITORY") {
    Some(repository) => repository,
    None => "hahatulk/termian",
};
const MANIFEST_ASSET: &str = "latest.json";
pub const FIRST_CHECK: Duration = Duration::from_secs(20);
pub const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_PACKAGE_BYTES: u64 = 256 * 1024 * 1024;
const PROGRESS_STEP: u64 = 256 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Channel {
    #[default]
    Latest,
    PreRelease,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum State {
    Disabled,
    #[default]
    Idle,
    Checking,
    Downloading,
    /// Verified and in place; starts on the next launch.
    Installed,
    Error,
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct Status {
    pub state: State,
    pub version: Option<String>,
    pub error: Option<String>,
    pub downloaded: Option<u64>,
    pub total: Option<u64>,
    /// When the last check finished (Unix ms); kept across states.
    pub checked_at: Option<u64>,
}

impl Status {
    pub fn set(&mut self, state: State, version: Option<String>, error: Option<String>) {
        let checked_at = if matches!(state, State::Idle | State::Error | State::Installed) { Some(unix_ms()) } else { self.checked_at };
        *self = Status { state, version, error, downloaded: None, total: None, checked_at };
    }
}

impl Status {
    /// Download progress in percent, when the server reported the size.
    pub fn percent(&self) -> Option<u64> {
        let total = self.total.filter(|t| *t > 0)?;
        Some((self.downloaded.unwrap_or(0) * 100 / total).min(100))
    }

    /// Settings → Version → «Обновления» text, as legacy words it; `time`
    /// formats `checked_at` (Unix ms).
    pub fn text(&self, time: impl Fn(u64) -> String) -> String {
        let mb = |bytes: u64| format!("{:.1}", bytes as f64 / 1_048_576.0);
        let version = self.version.as_ref().map_or_else(|| "Новая версия".to_owned(), |v| format!("Termian {v}"));
        let checked = self.checked_at.map(|at| format!(" Проверено: {}.", time(at))).unwrap_or_default();
        match self.state {
            State::Disabled => "Автообновление отключено: это локальная сборка или задан TERMIAN_DISABLE_UPDATES.".into(),
            State::Checking => "Проверка обновлений…".into(),
            State::Downloading => match self.percent() {
                Some(percent) => format!("Загрузка {version}: {percent}% ({} из {} МБ)", mb(self.downloaded.unwrap_or(0)), mb(self.total.unwrap_or(0))),
                None => format!("Загрузка {version}: {} МБ", mb(self.downloaded.unwrap_or(0))),
            },
            State::Installed => format!("{version} загружена и будет установлена при перезапуске. Открытые SSH-сессии при перезапуске закроются."),
            State::Error => format!("Не удалось проверить или загрузить обновление.{checked}"),
            State::Idle if self.checked_at.is_some() => format!("В выбранном канале нет более новой версии.{checked}"),
            State::Idle => "Обновления проверяются автоматически через 20 секунд после запуска и затем каждые 6 часов.".into(),
        }
    }
}

/// Only CI release builds update themselves: a local build keeps the crate
/// version and must not be replaced by the published release.
pub fn enabled() -> bool {
    option_env!("TERMIAN_RELEASE_VERSION").is_some() && std::env::var_os("TERMIAN_DISABLE_UPDATES").is_none()
}

pub fn current_version() -> semver::Version {
    let text = option_env!("TERMIAN_RELEASE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
    semver::Version::parse(text.trim_start_matches('v')).unwrap_or_else(|_| semver::Version::new(0, 0, 0))
}

fn unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_millis() as u64).unwrap_or(0)
}

#[derive(Deserialize)]
pub struct Release {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

fn newest_manifest(releases: Vec<Release>, current: &semver::Version, channel: Channel) -> Option<(semver::Version, url::Url)> {
    let prerelease = channel == Channel::PreRelease;
    releases
        .into_iter()
        .filter(|release| !release.draft && release.prerelease == prerelease)
        .filter_map(|release| {
            let version = semver::Version::parse(release.tag_name.strip_prefix('v')?).ok()?;
            let asset = release.assets.into_iter().find(|asset| asset.name == MANIFEST_ASSET)?;
            let url = url::Url::parse(&asset.browser_download_url).ok()?;
            (version.pre.is_empty() != prerelease && version.build.is_empty() && &version > current
                && url.scheme() == "https" && url.host_str() == Some("github.com"))
                .then_some((version, url))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
}

#[derive(Deserialize)]
struct Manifest {
    version: String,
    #[serde(default)]
    platforms: std::collections::HashMap<String, Platform>,
}

#[derive(Deserialize)]
struct Platform {
    signature: String,
    url: String,
}

/// One package of `latest.json` for this platform, not yet downloaded.
#[derive(Clone, PartialEq, Debug)]
pub struct Package {
    pub version: String,
    pub url: url::Url,
    pub signature: String,
    expected_file: String,
}

/// `latest.json` key of this build. The native client ships only the bundle
/// (macOS) and the portable EXE (Windows); there is no installer yet.
pub fn target() -> Option<&'static str> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("darwin-aarch64")
    } else if cfg!(all(windows, target_arch = "x86_64")) {
        Some("windows-x86_64")
    } else {
        None
    }
}

/// The package for `target` when the manifest announces a version above `current`.
pub fn select_package(manifest: &str, target: &str, current: &semver::Version) -> Result<Option<Package>, String> {
    let manifest: Manifest = serde_json::from_str(manifest).map_err(|error| format!("latest.json: {error}"))?;
    let version = semver::Version::parse(manifest.version.trim_start_matches('v')).map_err(|error| format!("latest.json: {error}"))?;
    if &version <= current {
        return Ok(None);
    }
    let platform = manifest.platforms.get(target).ok_or_else(|| format!("latest.json has no package for {target}"))?;
    let url = url::Url::parse(&platform.url).map_err(|error| format!("latest.json: {error}"))?;
    if url.scheme() != "https" {
        return Err("latest.json: the package must be downloaded over HTTPS".into());
    }
    let suffix = match target {
        "darwin-aarch64" => "mac-arm64.app.tar.gz",
        "windows-x86_64" => "win-x64.exe",
        _ => return Err("Пакет недоступен для автообновления на этой платформе".into()),
    };
    let expected_file = format!("termian-{version}-{suffix}");
    Ok(Some(Package { expected_file, version: version.to_string(), url, signature: platform.signature.clone() }))
}

fn base64_text(text: &str) -> Result<String, String> {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD.decode(text.trim()).map_err(|error| format!("signature: {error}"))?;
    String::from_utf8(bytes).map_err(|_| "signature: not UTF-8".to_owned())
}

/// Accept `bytes` only when the release key signed them for `version`, as
/// tauri-plugin-updater does with `requireSignedVersion`: the manifest is not
/// signed, so the signed version is what stops an old package passing as new.
pub fn verify(bytes: &[u8], signature: &str, version: &str, expected_file: &str) -> Result<(), String> {
    let key = minisign_verify::PublicKey::decode(&base64_text(PUBLIC_KEY)?).map_err(|error| format!("public key: {error}"))?;
    let signature = minisign_verify::Signature::decode(&base64_text(signature)?).map_err(|error| format!("signature: {error}"))?;
    key.verify(bytes, &signature, true).map_err(|_| "Подпись обновления не совпадает с ключом выпуска".to_owned())?;
    // The trusted comment is covered by the global signature verified above.
    let mut files = signature.trusted_comment().split('\t').filter_map(|field| field.strip_prefix("file:"));
    if files.next() != Some(expected_file) || files.next().is_some() {
        return Err("Подпись не соответствует пакету обновления для этой платформы".into());
    }
    let signed = signature.trusted_comment().split('\t').find_map(|field| field.strip_prefix("version:"));
    let Some(signed) = signed else { return Err("Подпись обновления не содержит версию".into()) };
    let parse = |text: &str| semver::Version::parse(text.trim().trim_start_matches('v')).ok();
    match (parse(signed), parse(version)) {
        (Some(signed), Some(announced)) if signed == announced => Ok(()),
        _ => Err(format!("Подпись выпуска {signed} не совпадает с объявленной версией {version}")),
    }
}

fn get(url: &str) -> Result<ureq::Response, ureq::Error> {
    ureq::get(url)
        .timeout(REQUEST_TIMEOUT)
        .set("accept", "application/vnd.github+json")
        .set("user-agent", "termian-updater")
        .call()
}

fn release_json<T: serde::de::DeserializeOwned>(response: ureq::Response) -> Result<T, String> {
    let body = response.into_string().map_err(|error| format!("GitHub releases: {error}"))?;
    serde_json::from_str(&body).map_err(|error| format!("GitHub releases: {error}"))
}

pub fn check(current: &semver::Version, channel: Channel) -> Result<Option<Package>, String> {
    let Some(target) = target() else { return Err("Автоматические обновления не поддерживаются на этой платформе".into()) };
    let base = format!("https://api.github.com/repos/{REPOSITORY}/releases");
    let mut releases = Vec::new();
    match channel {
        Channel::Latest => match get(&format!("{base}/latest")) {
            Ok(response) => releases.push(release_json(response)?),
            Err(ureq::Error::Status(404, _)) => return Ok(None),
            Err(error) => return Err(format!("GitHub releases: {error}")),
        },
        Channel::PreRelease => {
            for page in 1..=5 {
                let response = get(&format!("{base}?per_page=100&page={page}"))
                    .map_err(|error| format!("GitHub releases: {error}"))?;
                let has_next = response.header("Link").is_some_and(|links| links.contains("rel=\"next\""));
                releases.extend(release_json::<Vec<Release>>(response)?);
                if !has_next {
                    break;
                }
            }
        }
    }
    let Some((version, manifest)) = newest_manifest(releases, current, channel) else { return Ok(None) };
    let manifest = get(manifest.as_str()).map_err(|error| format!("latest.json: {error}"))?
        .into_string().map_err(|error| format!("latest.json: {error}"))?;
    package_for_release(&manifest, target, current, &version).map(Some)
}

fn package_for_release(manifest: &str, target: &str, current: &semver::Version, version: &semver::Version) -> Result<Package, String> {
    select_package(manifest, target, current)?
        .filter(|package| package.version == version.to_string())
        .ok_or_else(|| format!("latest.json: версия пакета не совпадает с тегом v{version}"))
}

/// Download and verify `package`; `progress(downloaded, total)` runs every 256 KB.
pub fn download(package: &Package, mut progress: impl FnMut(u64, Option<u64>)) -> Result<Vec<u8>, String> {
    let response = ureq::get(package.url.as_str())
        .timeout(Duration::from_secs(15 * 60))
        .set("user-agent", "termian-updater")
        .call()
        .map_err(|error| format!("Загрузка обновления: {error}"))?;
    let total = response.header("content-length").and_then(|length| length.parse().ok());
    if total.is_some_and(|size| size > MAX_PACKAGE_BYTES) {
        return Err("Пакет обновления превышает 256 МиБ".into());
    }
    let (mut reader, mut bytes, mut chunk, mut reported) = (response.into_reader(), Vec::new(), vec![0u8; 64 * 1024], 0u64);
    loop {
        let read = reader.read(&mut chunk).map_err(|error| format!("Загрузка обновления: {error}"))?;
        if read == 0 {
            break;
        }
        if bytes.len() as u64 + read as u64 > MAX_PACKAGE_BYTES {
            return Err("Пакет обновления превышает 256 МиБ".into());
        }
        bytes.extend_from_slice(&chunk[..read]);
        let downloaded = bytes.len() as u64;
        if downloaded - reported >= PROGRESS_STEP || total == Some(downloaded) {
            reported = downloaded;
            progress(downloaded, total);
        }
    }
    verify(&bytes, &package.signature, &package.version, &package.expected_file)?;
    Ok(bytes)
}

/// Put verified `bytes` in place of the running app.
pub fn install(bytes: &[u8]) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    if cfg!(target_os = "macos") {
        let bundle = bundle_path(&exe).ok_or("Обновление ставится только в собранный .app")?;
        install_bundle(&bundle, bytes).map_err(|error| format!("Установка обновления: {error}"))
    } else if cfg!(windows) {
        replace_exe(&exe, bytes).map_err(|error| format!("Установка обновления: {error}"))
    } else {
        Err("Автоматические обновления не поддерживаются на этой платформе".into())
    }
}

/// `Termian.app` for `Termian.app/Contents/MacOS/<exe>`.
pub fn bundle_path(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    (macos.file_name()? == "MacOS" && contents.file_name()? == "Contents" && bundle.extension()? == "app").then(|| bundle.to_path_buf())
}

/// Extract `Name.app/...` from the `.app.tar.gz` beside `bundle` and swap it in,
/// like tauri-plugin-updater: the archive's top folder is dropped, the staging
/// folder becomes the new bundle, and the previous bundle is removed only after
/// the new one is in place. A bundle the user cannot write is refused (the
/// plugin asks for an administrator password there).
pub fn install_bundle(bundle: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    let parent = bundle.parent().ok_or_else(|| Error::new(ErrorKind::NotFound, "no parent folder"))?;
    let staged = tempfile::Builder::new().prefix(".termian-update").tempdir_in(parent)?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    let mut binary = false;
    for entry in archive.entries()? {
        let mut entry = entry?;
        let inner: PathBuf = entry.path()?.iter().skip(1).collect();
        if inner.as_os_str().is_empty() {
            continue;
        }
        if inner.components().any(|part| !matches!(part, std::path::Component::Normal(_))) {
            return Err(Error::new(ErrorKind::InvalidData, "unsafe path in the update archive"));
        }
        binary |= inner.starts_with("Contents/MacOS") && entry.header().entry_type().is_file();
        let path = staged.path().join(&inner);
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        entry.unpack(&path)?;
    }
    if !binary {
        return Err(Error::new(ErrorKind::InvalidData, "the update archive has no app bundle"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(staged.path(), std::fs::Permissions::from_mode(0o755))?;
    }
    let staged = staged.keep();
    let backup = with_suffix(&staged, ".old");
    let result = std::fs::rename(bundle, &backup).and_then(|()| {
        std::fs::rename(&staged, bundle).inspect_err(|_| {
            let _ = std::fs::rename(&backup, bundle);
        })
    });
    if let Err(error) = result {
        let _ = std::fs::remove_dir_all(&staged);
        return Err(error);
    }
    let _ = std::fs::remove_dir_all(&backup);
    Ok(())
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    name.into()
}

/// Windows lets a running EXE be renamed but not overwritten: write the new one
/// beside it, move the running one to `.old`, then move the new one into place.
pub fn replace_exe(exe: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::{fs, io};
    if bytes.len() < 1_000_000 || !bytes.starts_with(b"MZ") {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "update is not a Windows executable"));
    }
    let (new, old) = (with_suffix(exe, ".new"), with_suffix(exe, ".old"));
    fs::write(&new, bytes)?;
    let _ = fs::remove_file(&old);
    if let Err(error) = fs::rename(exe, &old) {
        let _ = fs::remove_file(&new);
        return Err(error);
    }
    if let Err(error) = fs::rename(&new, exe) {
        let _ = fs::rename(&old, exe);
        return Err(error);
    }
    Ok(())
}

/// The previous EXE stays locked while it runs; remove it on a later start.
pub fn remove_replaced_exe() {
    if cfg!(windows) {
        if let Ok(exe) = std::env::current_exe() {
            for suffix in [".old", ".new"] {
                let _ = std::fs::remove_file(with_suffix(&exe, suffix));
            }
        }
    }
}

/// Start the installed version: the bundle through LaunchServices, the EXE directly.
pub fn relaunch() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut command = match bundle_path(&exe).filter(|_| cfg!(target_os = "macos")) {
        Some(bundle) => {
            let mut open = std::process::Command::new("/usr/bin/open");
            open.arg("-n").arg(bundle);
            open
        }
        None => std::process::Command::new(exe),
    };
    command.spawn().map(|_| ()).map_err(|error| format!("Перезапуск: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Signed by the release key over `termian-updater-key-check` with
    /// `version:1.0.0` (the legacy updater test uses the same signature).
    const KEY_CHECK_SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVSR2VSYWtBRmhUMS9kNndVSU5rb2dqd0IybGRvanlLTzQ0TGxpaEZBbTBMQ01vZWp4QWlVdkJUTWNaRzJBWlIzZWJaUEpOMHdzVmlKc0xQRTA1dUI5TXRmbG1WZ0x3SkFRPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk0MjU5OTYyCWZpbGU6a2V5Y2hlY2sudHh0CXZlcnNpb246MS4wLjAKK1BLdjEzYVJGSG11Smd2cDM3L3NKTVovMVQ0UHR4ZEFUd0dMa3A1aTE3Ly9rdUZxZFRucUFiajduY2s3RFVEektVMTM2c2pFbnFLQWZERWVhZWNmQkE9PQo=";

    #[test]
    fn accepts_only_release_signatures_bound_to_the_version() {
        let data = b"termian-updater-key-check";
        verify(data, KEY_CHECK_SIGNATURE, "1.0.0", "keycheck.txt").unwrap();
        verify(data, KEY_CHECK_SIGNATURE, "v1.0.0", "keycheck.txt").unwrap();
        assert!(verify(b"tampered", KEY_CHECK_SIGNATURE, "1.0.0", "keycheck.txt").unwrap_err().contains("Подпись"));
        assert!(verify(data, KEY_CHECK_SIGNATURE, "1.2.0", "keycheck.txt").unwrap_err().contains("не совпадает с объявленной"));
        assert!(verify(data, "not base64!", "1.0.0", "keycheck.txt").is_err());
        assert!(verify(data, KEY_CHECK_SIGNATURE, "1.0.0", "other.exe").is_err());
    }

    fn release(tag: &str, prerelease: bool, draft: bool, manifest: Option<&str>) -> Release {
        Release {
            tag_name: tag.into(),
            draft,
            prerelease,
            assets: manifest
                .map(|url| vec![Asset { name: MANIFEST_ASSET.into(), browser_download_url: url.into() }])
                .unwrap_or_default(),
        }
    }

    #[test]
    fn updates_stay_in_the_selected_channel() {
        let current = semver::Version::new(1, 2, 0);
        let url = |tag: &str| format!("https://github.com/o/r/releases/download/{tag}/latest.json");
        let releases = || vec![
            release("v1.1.0", false, false, Some(&url("v1.1.0"))),
            release("v1.4.0", false, true, Some(&url("v1.4.0"))),
            release("v1.3.0", false, false, Some(&url("v1.3.0"))),
            release("v1.3.5", false, false, None),
            release("v1.8.0", true, false, Some(&url("v1.8.0"))),
            release("v2.0.0-beta.2", true, false, Some(&url("v2.0.0-beta.2"))),
            release("v2.0.0-beta.10", true, false, Some(&url("v2.0.0-beta.10"))),
            release("v3.0.0-beta.1", true, true, Some(&url("v3.0.0-beta.1"))),
            release("v4.0.0-beta.1", false, false, Some(&url("v4.0.0-beta.1"))),
            release("v1.3.1", false, false, Some("https://example.com/latest.json")),
            release("nightly", true, false, Some(&url("nightly"))),
        ];
        let find = |channel, installed: &semver::Version| {
            newest_manifest(releases(), installed, channel).map(|(version, _)| version.to_string())
        };
        assert_eq!(find(Channel::Latest, &current).as_deref(), Some("1.3.0"));
        assert_eq!(find(Channel::PreRelease, &current).as_deref(), Some("2.0.0-beta.10"));
        let beta = semver::Version::parse("2.0.0-beta.10").unwrap();
        assert_eq!(find(Channel::Latest, &beta), None);
        assert_eq!(find(Channel::PreRelease, &beta), None);
        let body = r#"{"tag_name":"v2.0.0","draft":false,"prerelease":false,"assets":[{"name":"latest.json","browser_download_url":"https://github.com/o/r/releases/download/v2.0.0/latest.json"}]}"#;
        assert!(newest_manifest(vec![serde_json::from_str(body).unwrap()], &beta, Channel::Latest).is_some());
    }

    #[test]
    fn package_for_this_platform_is_read_from_latest_json() {
        let manifest = r#"{"version":"1.3.0","notes":"","pub_date":"2026-11-01T00:00:00.000Z","platforms":{
            "darwin-aarch64":{"signature":"c2ln","url":"https://github.com/o/r/releases/download/v1.3.0/termian-1.3.0-mac-arm64.app.tar.gz"},
            "windows-x86_64":{"signature":"d2lu","url":"https://github.com/o/r/releases/download/v1.3.0/termian-1.3.0-win-x64.exe"},
            "windows-x86_64-nsis":{"signature":"bnNpcw==","url":"https://github.com/o/r/releases/download/v1.3.0/termian-1.3.0-win-x64-setup.exe"}}}"#;
        let current = semver::Version::new(1, 2, 0);
        let mac = select_package(manifest, "darwin-aarch64", &current).unwrap().unwrap();
        assert_eq!((mac.version.as_str(), mac.signature.as_str()), ("1.3.0", "c2ln"));
        let beta = semver::Version::parse("1.3.0-beta.1").unwrap();
        assert!(package_for_release(manifest, "darwin-aarch64", &current, &beta).is_err());
        let beta_manifest = manifest.replace("1.3.0", "1.3.0-beta.1");
        assert_eq!(package_for_release(&beta_manifest, "darwin-aarch64", &current, &beta).unwrap().version, beta.to_string());
        assert!(mac.url.as_str().ends_with("mac-arm64.app.tar.gz"));
        let windows = select_package(manifest, "windows-x86_64", &current).unwrap().unwrap();
        assert!(windows.url.as_str().ends_with("win-x64.exe"), "portable, not the installer");
        assert_eq!(select_package(manifest, "darwin-aarch64", &semver::Version::new(1, 3, 0)).unwrap(), None);
        assert!(select_package(manifest, "linux-x86_64", &current).is_err());
        assert!(select_package(&manifest.replace("https://github.com/o/r/releases/download/v1.3.0/termian-1.3.0-mac", "http://x/mac"), "darwin-aarch64", &current).is_err());
        assert!(target().is_none() || manifest.contains(target().unwrap()));
    }

    fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast()));
        for (path, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, path, *data).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn bundle_is_swapped_and_bad_archives_leave_it_alone() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("Termian.app");
        std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        std::fs::write(bundle.join("Contents/MacOS/termian-native"), b"old").unwrap();
        std::fs::write(bundle.join("Contents/stale.txt"), b"gone after update").unwrap();
        assert_eq!(bundle_path(&bundle.join("Contents/MacOS/termian-native")), Some(bundle.clone()));
        assert_eq!(bundle_path(Path::new("/usr/local/bin/termian-native")), None);

        assert!(install_bundle(&bundle, &archive(&[("Termian.app/Contents/Info.plist", b"x")])).is_err(), "no binary");
        assert!(install_bundle(&bundle, b"not a tarball").is_err());
        assert_eq!(std::fs::read(bundle.join("Contents/MacOS/termian-native")).unwrap(), b"old");

        let update = archive(&[("Termian.app/Contents/Info.plist", b"plist"), ("Termian.app/Contents/MacOS/termian-native", b"new")]);
        install_bundle(&bundle, &update).unwrap();
        assert_eq!(std::fs::read(bundle.join("Contents/MacOS/termian-native")).unwrap(), b"new");
        assert!(!bundle.join("Contents/stale.txt").exists());
        let leftovers: Vec<_> = std::fs::read_dir(dir.path()).unwrap().flatten().map(|entry| entry.file_name()).collect();
        assert_eq!(leftovers, vec![std::ffi::OsString::from("Termian.app")], "staging and backup removed");
    }

    #[test]
    fn exe_is_replaced_in_place_and_garbage_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("termian.exe");
        std::fs::write(&exe, b"old").unwrap();
        assert!(replace_exe(&exe, b"not an exe").is_err());
        assert_eq!(std::fs::read(&exe).unwrap(), b"old");
        assert!(!with_suffix(&exe, ".new").exists());
        let mut next = b"MZ".to_vec();
        next.resize(1_000_001, 0);
        replace_exe(&exe, &next).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), next);
        assert_eq!(std::fs::read(with_suffix(&exe, ".old")).unwrap(), b"old");
    }

    #[test]
    fn version_tab_text_follows_the_state() {
        let time = |_| "13.11.2026, 10:00".to_owned();
        let mut status = Status::default();
        assert!(status.text(time).starts_with("Обновления проверяются автоматически"));
        status = Status { state: State::Downloading, version: Some("1.3.0".into()), downloaded: Some(3 * 1_048_576), total: Some(12 * 1_048_576), ..Status::default() };
        assert_eq!(status.text(time), "Загрузка Termian 1.3.0: 25% (3.0 из 12.0 МБ)");
        status.total = None;
        assert_eq!(status.text(time), "Загрузка Termian 1.3.0: 3.0 МБ");
        status = Status { state: State::Idle, checked_at: Some(1), ..Status::default() };
        assert_eq!(status.text(time), "В выбранном канале нет более новой версии. Проверено: 13.11.2026, 10:00.");
        status.state = State::Installed;
        assert!(status.text(time).starts_with("Новая версия загружена"));
        assert!(Status { state: State::Disabled, ..Status::default() }.text(time).contains("TERMIAN_DISABLE_UPDATES"));
    }

    #[test]
    fn status_keeps_the_last_check_time() {
        let mut status = Status::default();
        status.set(State::Checking, None, None);
        assert_eq!(status.checked_at, None);
        status.set(State::Idle, None, None);
        let checked = status.checked_at.unwrap();
        status.set(State::Downloading, Some("1.3.0".into()), None);
        assert_eq!(status.checked_at, Some(checked));
        assert!(!enabled() || option_env!("TERMIAN_RELEASE_VERSION").is_some());
        assert_eq!(current_version(), semver::Version::parse(option_env!("TERMIAN_RELEASE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))).unwrap());
    }
}

/// Against the published releases: `cargo test --lib updater::live -- --ignored`.
/// Downloads and verifies the real package for this platform; installs nothing.
#[cfg(test)]
mod live {
    #[test]
    #[ignore = "network: GitHub releases"]
    fn published_package_verifies_with_the_release_key() {
        let package = super::check(&semver::Version::new(0, 0, 1), super::Channel::Latest).unwrap().expect("a published release with latest.json");
        let bytes = super::download(&package, |_, _| {}).unwrap();
        eprintln!("verified {} ({} bytes) from {}", package.version, bytes.len(), package.url);
        assert!(super::verify(&bytes[1..], &package.signature, &package.version, &package.expected_file).is_err());
    }
}
