use base64::{engine::general_purpose::STANDARD, Engine};
use minisign::{PublicKeyBox, SecretKeyBox};
use semver::Version;
use serde_json::{json, Map};
use std::{
    collections::BTreeSet, env, error::Error, fs, fs::File, io::Write, path::Path, process::Command,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn release_version(text: &str) -> Result<Version> {
    let version = Version::parse(text.strip_prefix('v').unwrap_or(text))?;
    if !version.build.is_empty() {
        return Err("release versions cannot contain build metadata".into());
    }
    Ok(version)
}

fn git(args: &[&str]) -> Result<String> {
    let out = Command::new("git").args(args).output()?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned().into());
    }
    Ok(String::from_utf8(out.stdout)?.trim().to_owned())
}

fn tagged(text: &str) -> BTreeSet<Version> {
    text.lines()
        .filter(|tag| tag.starts_with('v'))
        .filter_map(|tag| release_version(tag).ok())
        .collect()
}

fn choose(
    tags: &BTreeSet<Version>,
    at_head: &BTreeSet<Version>,
    requested: &str,
) -> Result<Version> {
    let highest_stable = tags.iter().rev().find(|version| version.pre.is_empty());
    if !requested.is_empty() {
        let version = release_version(requested)?;
        let highest = if version.pre.is_empty() {
            highest_stable
        } else {
            tags.last()
        };
        if highest.is_some_and(|top| &version < top)
            || (tags.contains(&version) && !at_head.contains(&version))
        {
            return Err(format!(
                "v{version} is older than the latest tag or belongs to another commit"
            )
            .into());
        }
        return Ok(version);
    }
    if let Some(version) = highest_stable.filter(|version| at_head.contains(*version)) {
        return Ok(version.clone());
    }
    match highest_stable {
        None => Ok(Version::new(1, 0, 0)),
        Some(version) => Ok(Version::new(
            version.major,
            version.minor,
            version
                .patch
                .checked_add(1)
                .ok_or("patch version overflow")?,
        )),
    }
}

fn prepare() -> Result<()> {
    let tags = tagged(&git(&["tag", "--list"])?);
    let at_head = tagged(&git(&["tag", "--points-at", "HEAD"])?);
    let version = choose(
        &tags,
        &at_head,
        &env::var("REQUESTED_VERSION").unwrap_or_default(),
    )?;
    let sha = git(&["rev-parse", "HEAD"])?;
    let previous = tags
        .range(..version.clone())
        .rev()
        .filter(|tag| !version.pre.is_empty() || tag.pre.is_empty())
        .find(|tag| {
            Command::new("git")
                .args(["merge-base", "--is-ancestor", &format!("v{tag}"), "HEAD"])
                .output()
                .is_ok_and(|out| out.status.success())
        });
    let range = previous.map_or("HEAD".to_owned(), |tag| format!("v{tag}..HEAD"));
    let log = git(&["log", "--format=- `%h` %s", &range, "--"])?;
    let mut output = fs::OpenOptions::new()
        .append(true)
        .open(env::var_os("GITHUB_OUTPUT").ok_or("missing GITHUB_OUTPUT")?)?;
    let prerelease = !version.pre.is_empty();
    writeln!(output, "version={version}\ntag=v{version}\nsha={sha}\nprerelease={prerelease}\nchangelog<<TERM_{sha}\n## Termian {version}\n\n{log}\nTERM_{sha}")?;
    Ok(())
}

fn packages(version: &Version) -> [(&'static str, String); 3] {
    [
        (
            "darwin-aarch64",
            format!("termian-{version}-mac-arm64.app.tar.gz"),
        ),
        ("windows-x86_64", format!("termian-{version}-win-x64.exe")),
        (
            "windows-x86_64-nsis",
            format!("termian-{version}-win-x64-setup.exe"),
        ),
    ]
}

fn exact_assets(dir: &Path, version: &Version, files: &[(&str, String); 3]) -> Result<()> {
    let mut expected: BTreeSet<String> = files.iter().map(|(_, name)| name.clone()).collect();
    expected.insert(format!("termian-{version}-mac-arm64.dmg"));
    expected.insert(format!("termian-{version}-android.apk"));
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() || entry.metadata()?.len() == 0 {
            return Err(format!("invalid release asset: {:?}", entry.file_name()).into());
        }
        actual.insert(
            entry
                .file_name()
                .into_string()
                .map_err(|_| "invalid release filename")?,
        );
    }
    if actual != expected {
        return Err(format!("expected assets {expected:?}, found {actual:?}").into());
    }
    Ok(())
}

fn sign_assets(
    dir: &Path,
    version: &Version,
    repo: &str,
    notes: &str,
    pk: &minisign::PublicKey,
    sk: &minisign::SecretKey,
) -> Result<()> {
    let files = packages(version);
    exact_assets(dir, version, &files)?;
    let mut platforms = Map::new();
    for (target, name) in &files {
        let comment = format!("file:{name}\tversion:{version}");
        let signature = minisign::sign(
            Some(pk),
            sk,
            File::open(dir.join(name))?,
            Some(&comment),
            None,
        )?;
        minisign::verify(
            pk,
            &signature,
            File::open(dir.join(name))?,
            true,
            false,
            false,
        )?;
        platforms.insert(
            (*target).to_owned(),
            json!({
                "signature": STANDARD.encode(signature.into_string()),
                "url": format!("https://github.com/{repo}/releases/download/v{version}/{name}"),
            }),
        );
    }
    for (target, name) in &files {
        fs::write(
            dir.join(format!("{name}.sig")),
            platforms[*target]["signature"]
                .as_str()
                .ok_or("missing signature")?,
        )?;
    }
    let date = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()?;
    if !date.status.success() {
        return Err("could not read release date".into());
    }
    let manifest = json!({
        "version": version.to_string(),
        "notes": notes,
        "pub_date": String::from_utf8(date.stdout)?.trim(),
        "platforms": platforms,
    });
    fs::write(
        dir.join("latest.json"),
        serde_json::to_string_pretty(&manifest)? + "\n",
    )?;
    Ok(())
}

fn publish(dir: &Path) -> Result<()> {
    let version = release_version(&env::var("TERMIAN_RELEASE_VERSION")?)?;
    let repository = env::var("GITHUB_REPOSITORY")?;
    if !repository.split_once('/').is_some_and(|(owner, repo)| {
        [owner, repo].iter().all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
        })
    }) {
        return Err("invalid GitHub repository".into());
    }
    let encoded =
        env::var("TERMIAN_SIGNING_PRIVATE_KEY").map_err(|_| "missing release signing key")?;
    let key = STANDARD
        .decode(encoded.trim())
        .map_err(|_| "invalid base64 release signing key")?;
    let sk = SecretKeyBox::from_string(
        std::str::from_utf8(&key).map_err(|_| "invalid release signing key")?,
    )?
    .into_secret_key(Some(
        env::var("TERMIAN_SIGNING_PRIVATE_KEY_PASSWORD").unwrap_or_default(),
    ))?;
    let pk = STANDARD.decode(include_str!("../../../assets/app/update-public-key").trim())?;
    let pk = PublicKeyBox::from_string(std::str::from_utf8(&pk)?)?.into_public_key()?;
    sign_assets(
        dir,
        &version,
        &repository,
        &env::var("RELEASE_NOTES").unwrap_or_default(),
        &pk,
        &sk,
    )
}

fn main() -> Result<()> {
    match env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [command] if command == "prepare" => prepare(),
        [command, dir] if command == "sign" => publish(Path::new(dir)),
        _ => Err("expected prepare or sign <release-directory>".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_packages_use_the_client_format() {
        let dir = tempfile::tempdir().unwrap();
        let version = Version::parse("1.2.3-beta.1").unwrap();
        let files = packages(&version);
        let pair = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        let public = minisign_verify::PublicKey::from_base64(&pair.pk.to_base64()).unwrap();
        let package_bytes = b"package fixture";

        for (_, name) in &files {
            fs::write(dir.path().join(name), package_bytes).unwrap();
        }
        assert!(sign_assets(dir.path(), &version, "owner/repo", "", &pair.pk, &pair.sk).is_err());
        assert!(!dir.path().join("latest.json").exists());
        fs::write(
            dir.path().join(format!("termian-{version}-mac-arm64.dmg")),
            b"image",
        )
        .unwrap();
        assert!(sign_assets(dir.path(), &version, "owner/repo", "", &pair.pk, &pair.sk).is_err());
        fs::write(
            dir.path().join(format!("termian-{version}-android.apk")),
            b"apk fixture",
        )
        .unwrap();

        sign_assets(dir.path(), &version, "owner/repo", "", &pair.pk, &pair.sk).unwrap();
        let manifest: serde_json::Value =
            serde_json::from_reader(File::open(dir.path().join("latest.json")).unwrap()).unwrap();
        for (target, name) in files {
            let sig = manifest["platforms"][target]["signature"].as_str().unwrap();
            assert_eq!(
                fs::read_to_string(dir.path().join(format!("{name}.sig"))).unwrap(),
                sig
            );
            let decoded = STANDARD.decode(sig).unwrap();
            let signature =
                minisign_verify::Signature::decode(std::str::from_utf8(&decoded).unwrap()).unwrap();
            assert_eq!(
                signature.trusted_comment().split('\t').last(),
                Some("version:1.2.3-beta.1")
            );
            public.verify(package_bytes, &signature, true).unwrap();
        }
    }

    #[test]
    fn release_version_respects_existing_tags() {
        let tags = tagged("nightly\nv1.0.9\nv1.0.12\nv2.0.0-rc.1\n");
        let untagged = BTreeSet::new();
        assert!(choose(&tags, &untagged, "2.0.0-rc.1").is_err());
        assert!(choose(&tags, &untagged, "2.0.0-beta.9").is_err());
        assert!(choose(&tags, &untagged, "2.0.0+build.1").is_err());
        assert_eq!(
            choose(&tags, &untagged, "2.0.0-rc.2").unwrap(),
            Version::parse("2.0.0-rc.2").unwrap()
        );
        assert_eq!(
            choose(&tags, &untagged, "").unwrap(),
            Version::new(1, 0, 13)
        );
        assert!(choose(&tags, &untagged, "1.0.12").is_err());
        assert!(choose(&tags, &untagged, "1.0.9").is_err());
        assert_eq!(
            choose(&tags, &tagged("v1.0.12"), "").unwrap(),
            Version::new(1, 0, 12)
        );
        assert_eq!(
            choose(&tags, &untagged, "v2.0.0").unwrap(),
            Version::new(2, 0, 0)
        );
    }
}
