// Automatic updates from this repository's GitHub releases.
//
// Every release publishes `latest.json` with minisign signatures bound to the
// version (`tauri signer sign --app-version`). The newest stable release that
// carries it is found through the GitHub API (releases are published as
// prereleases, which `/releases/latest` skips), then tauri-plugin-updater
// downloads and verifies the package against the public key in tauri.conf.json.
//
// macOS: the plugin swaps the signed .app bundle in place.
// Windows portable EXE (manifest key `windows-x86_64`): the plugin installs
// only NSIS/MSI, so the running EXE is renamed aside and the new one takes its
// path; the termian-data/ profile next to it is untouched.
// Windows installed app (`windows-x86_64-nsis`, the plugin picks it from the
// bundle type stamped into the binary): the verified setup.exe is kept and
// runs in passive mode when the user restarts or quits, because the plugin
// exits the app as soon as the installer starts.
// Either way the new version starts on the next launch: the user chooses when
// to restart, so open SSH sessions are never cut off by an update.
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};

const REPOSITORY: &str = match option_env!("TERMIAN_UPDATE_REPOSITORY") {
    Some(repository) => repository,
    None => "hahatulk/termius",
};
const MANIFEST_ASSET: &str = "latest.json";
const FIRST_CHECK: Duration = Duration::from_secs(20);
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
pub const STATUS_EVENT: &str = "termian://update-status";

#[derive(Clone, Default, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    /// disabled | idle | checking | downloading | installed | error
    pub state: &'static str,
    pub version: Option<String>,
    pub error: Option<String>,
    /// Download progress in bytes (`total` when the server reports it).
    pub downloaded: Option<u64>,
    pub total: Option<u64>,
    /// When the last check finished (Unix ms); kept across states.
    pub checked_at: Option<u64>,
}

/// One check at a time: the timer and the Settings button share it.
static CHECKING: AtomicBool = AtomicBool::new(false);
const PROGRESS_STEP: u64 = 256 * 1024;

#[derive(Default)]
pub struct UpdateState(Mutex<UpdateStatus>);

/// A verified installer update waiting for restart or quit (installed build).
static PENDING_INSTALLER: Mutex<Option<(Update, Vec<u8>)>> = Mutex::new(None);

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// Only CI release builds update themselves: a local `npm run build:*` keeps
/// version 1.0.0 and must not be replaced by the published release.
fn enabled() -> bool {
    option_env!("TERMIAN_RELEASE_VERSION").is_some() && std::env::var_os("TERMIAN_DISABLE_UPDATES").is_none()
}

fn unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_millis() as u64).unwrap_or(0)
}

/// Every window listens: the notice in the main window and Settings → Version.
fn publish<R: Runtime>(app: &AppHandle<R>, change: impl FnOnce(&mut UpdateStatus)) {
    let status = match app.state::<UpdateState>().0.lock() {
        Ok(mut current) => {
            change(&mut current);
            current.clone()
        }
        Err(_) => return,
    };
    let _ = app.emit(STATUS_EVENT, status);
}

fn set<R: Runtime>(app: &AppHandle<R>, state: &'static str, version: Option<String>, error: Option<String>) {
    publish(app, |status| {
        let checked_at = if matches!(state, "idle" | "error" | "installed") { Some(unix_ms()) } else { status.checked_at };
        *status = UpdateStatus { state, version, error, downloaded: None, total: None, checked_at };
    });
}

pub fn start<R: Runtime>(app: &AppHandle<R>) {
    remove_replaced_exe();
    if !enabled() {
        set(app, "disabled", None, None);
        return;
    }
    publish(app, |status| status.state = "idle");
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            // Installed: nothing more to do until the user restarts.
            if run_check(&app).await {
                return;
            }
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}

/// Returns true once an update is installed and waits for a restart.
async fn run_check<R: Runtime>(app: &AppHandle<R>) -> bool {
    if CHECKING.swap(true, Ordering::SeqCst) {
        return false;
    }
    let installed = matches!(app.state::<UpdateState>().0.lock().map(|status| status.state), Ok("installed"));
    let result = if installed { Ok(true) } else { check_and_install(app).await };
    CHECKING.store(false, Ordering::SeqCst);
    match result {
        Ok(installed) => {
            if !installed {
                set(app, "idle", None, None);
            }
            installed
        }
        Err(error) => {
            eprintln!("[Termian] Update failed: {error}");
            set(app, "error", None, Some(error));
            false
        }
    }
}

async fn check_and_install<R: Runtime>(app: &AppHandle<R>) -> Result<bool, String> {
    set(app, "checking", None, None);
    let current = app.package_info().version.clone();
    let manifest = tauri::async_runtime::spawn_blocking(move || latest_manifest(&current))
        .await
        .map_err(|error| error.to_string())??;
    let Some(manifest) = manifest else { return Ok(false) };
    let update = app
        .updater_builder()
        .endpoints(vec![manifest])
        .and_then(|builder| builder.build())
        .map_err(|error| error.to_string())?
        .check()
        .await
        .map_err(|error| error.to_string())?;
    let Some(update) = update else { return Ok(false) };
    set(app, "downloading", Some(update.version.clone()), None);
    let (mut downloaded, mut reported) = (0u64, 0u64);
    // download() rejects bytes whose signature or signed version do not match.
    let bytes = update
        .download(
            |chunk, total| {
                downloaded += chunk as u64;
                if downloaded - reported >= PROGRESS_STEP || total == Some(downloaded) {
                    reported = downloaded;
                    publish(app, |status| {
                        status.downloaded = Some(downloaded);
                        status.total = total;
                    });
                }
            },
            || {},
        )
        .await
        .map_err(|error| error.to_string())?;
    install(&update, &bytes)?;
    set(app, "installed", Some(update.version.clone()), None);
    Ok(true)
}

fn latest_manifest(current: &semver::Version) -> Result<Option<url::Url>, String> {
    let body = ureq::get(&format!("https://api.github.com/repos/{REPOSITORY}/releases?per_page=30"))
        .timeout(REQUEST_TIMEOUT)
        .set("accept", "application/vnd.github+json")
        .set("user-agent", "termian-updater")
        .call()
        .map_err(|error| format!("GitHub releases: {error}"))?
        .into_string()
        .map_err(|error| format!("GitHub releases: {error}"))?;
    let releases: Vec<Release> = serde_json::from_str(&body).map_err(|error| format!("GitHub releases: {error}"))?;
    Ok(newest_manifest(releases, current))
}

/// The manifest of the highest stable version above `current`, if any.
fn newest_manifest(releases: Vec<Release>, current: &semver::Version) -> Option<url::Url> {
    releases
        .into_iter()
        .filter(|release| !release.draft)
        .filter_map(|release| {
            let version = semver::Version::parse(release.tag_name.strip_prefix('v')?).ok()?;
            let asset = release.assets.into_iter().find(|asset| asset.name == MANIFEST_ASSET)?;
            let url = url::Url::parse(&asset.browser_download_url).ok()?;
            (version.pre.is_empty() && &version > current && url.scheme() == "https" && url.host_str() == Some("github.com"))
                .then_some((version, url))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, url)| url)
}

#[cfg(target_os = "macos")]
fn install(update: &Update, bytes: &[u8]) -> Result<(), String> {
    update.install(bytes).map_err(|error| error.to_string())
}

#[cfg(windows)]
fn install(update: &Update, bytes: &[u8]) -> Result<(), String> {
    if crate::installed_build() {
        let mut pending = PENDING_INSTALLER.lock().map_err(|error| error.to_string())?;
        *pending = Some((update.clone(), bytes.to_vec()));
        return Ok(());
    }
    replace_exe(&std::env::current_exe().map_err(|error| error.to_string())?, bytes).map_err(|error| error.to_string())
}

/// Run a pending installer update. On success the plugin starts the NSIS
/// setup (passive, `/UPDATE`, relaunching the app) and exits this process.
pub fn install_pending() {
    let pending = PENDING_INSTALLER.lock().ok().and_then(|mut pending| pending.take());
    if let Some((update, bytes)) = pending {
        if let Err(error) = update.install(&bytes) {
            eprintln!("[Termian] Installer update failed: {error}");
        }
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
fn install(_update: &Update, _bytes: &[u8]) -> Result<(), String> {
    Err("Automatic updates are not supported on this platform".into())
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    name.into()
}

/// Windows lets a running EXE be renamed but not overwritten: write the new one
/// beside it, move the running one to `.old`, then move the new one into place.
#[cfg_attr(not(windows), allow(dead_code))]
fn replace_exe(exe: &Path, bytes: &[u8]) -> std::io::Result<()> {
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
fn remove_replaced_exe() {
    #[cfg(windows)]
    if let Ok(exe) = std::env::current_exe() {
        for suffix in [".old", ".new"] {
            let _ = std::fs::remove_file(with_suffix(&exe, suffix));
        }
    }
}

#[tauri::command]
pub fn update_status(state: tauri::State<'_, UpdateState>) -> Result<UpdateStatus, String> {
    state.0.lock().map(|status| status.clone()).map_err(|error| error.to_string())
}

/// Settings → Version → «Проверить обновления»: check (and download) now.
/// Progress and the result arrive as status events.
#[tauri::command]
pub fn update_check<R: Runtime>(app: AppHandle<R>, state: tauri::State<'_, UpdateState>) -> Result<UpdateStatus, String> {
    if !enabled() {
        return Err("Обновления отключены в этой сборке".into());
    }
    let current = state.0.lock().map_err(|error| error.to_string())?.clone();
    if current.state != "installed" && !CHECKING.load(Ordering::SeqCst) {
        tauri::async_runtime::spawn(async move {
            run_check(&app).await;
        });
    }
    Ok(current)
}

#[tauri::command]
pub fn update_restart<R: Runtime>(app: AppHandle<R>, state: tauri::State<'_, UpdateState>) -> Result<(), String> {
    if state.0.lock().map_err(|error| error.to_string())?.state != "installed" {
        return Err("No installed update is waiting for a restart".into());
    }
    // Installed build: the setup restarts the app after installing.
    install_pending();
    app.restart()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, draft: bool, manifest: Option<&str>) -> Release {
        Release {
            tag_name: tag.into(),
            draft,
            assets: manifest
                .map(|url| vec![Asset { name: MANIFEST_ASSET.into(), browser_download_url: url.into() }])
                .unwrap_or_default(),
        }
    }

    /// Signed by the release key (TAURI_SIGNING_PRIVATE_KEY): proves the
    /// public key in tauri.conf.json belongs to it. Regenerate after a key change.
    const KEY_CHECK_SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVSR2VSYWtBRmhUMS9kNndVSU5rb2dqd0IybGRvanlLTzQ0TGxpaEZBbTBMQ01vZWp4QWlVdkJUTWNaRzJBWlIzZWJaUEpOMHdzVmlKc0xQRTA1dUI5TXRmbG1WZ0x3SkFRPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk0MjU5OTYyCWZpbGU6a2V5Y2hlY2sudHh0CXZlcnNpb246MS4wLjAKK1BLdjEzYVJGSG11Smd2cDM3L3NKTVovMVQ0UHR4ZEFUd0dMa3A1aTE3Ly9rdUZxZFRucUFiajduY2s3RFVEektVMTM2c2pFbnFLQWZERWVhZWNmQkE9PQo=";

    #[test]
    fn configured_public_key_accepts_the_release_signing_key() {
        use base64::Engine as _;
        let config: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let decode = |text: &str| String::from_utf8(base64::engine::general_purpose::STANDARD.decode(text).unwrap()).unwrap();
        let key = minisign_verify::PublicKey::decode(&decode(config["plugins"]["updater"]["pubkey"].as_str().unwrap())).unwrap();
        let signature = minisign_verify::Signature::decode(&decode(KEY_CHECK_SIGNATURE)).unwrap();
        key.verify(b"termian-updater-key-check", &signature, false).unwrap();
        assert!(key.verify(b"tampered", &signature, false).is_err());
    }

    #[test]
    fn newest_signed_release_above_current_is_chosen() {
        let current = semver::Version::new(1, 2, 0);
        let url = |tag: &str| format!("https://github.com/hahatulk/termius/releases/download/{tag}/latest.json");
        let releases = vec![
            release("v1.1.0", false, Some(&url("v1.1.0"))),
            release("v1.4.0", true, Some(&url("v1.4.0"))),
            release("v1.3.0", false, Some(&url("v1.3.0"))),
            release("v1.3.5", false, None),
            release("v2.0.0-beta.1", false, Some(&url("v2.0.0-beta.1"))),
            release("v1.3.1", false, Some("https://example.com/latest.json")),
            release("nightly", false, Some(&url("nightly"))),
        ];
        assert_eq!(newest_manifest(releases, &current).unwrap().as_str(), url("v1.3.0"));
        assert!(newest_manifest(vec![release("v1.2.0", false, Some(&url("v1.2.0")))], &current).is_none());
    }

    #[test]
    fn exe_is_replaced_in_place_and_garbage_is_rejected() {
        let dir = std::env::temp_dir().join(format!("termian-update-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("termian.exe");
        std::fs::write(&exe, b"old").unwrap();
        assert!(replace_exe(&exe, b"not an exe").is_err());
        assert_eq!(std::fs::read(&exe).unwrap(), b"old");
        assert!(!with_suffix(&exe, ".new").exists());
        let mut next = b"MZ".to_vec();
        next.resize(1_000_001, 0);
        replace_exe(&exe, &next).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), next);
        assert_eq!(std::fs::read(with_suffix(&exe, ".old")).unwrap(), b"old");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
