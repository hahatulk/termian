use std::{env, path::PathBuf, process::Command};

fn xcrun(args: &[&str]) -> String {
    let output = Command::new("xcrun")
        .args(args)
        .output()
        .expect("run xcrun");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("xcrun output")
        .trim()
        .into()
}

fn main() {
    println!("cargo:rerun-if-changed=src/store/macos.swift");
    println!("cargo:rerun-if-env-changed=MACOSX_DEPLOYMENT_TARGET");
    match env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("windows") => {
            let assets =
                PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
                    .join("assets/app");
            println!("cargo:rerun-if-changed=assets/app/icon.ico");
            println!("cargo:rerun-if-changed=assets/app/termian.rc");
            embed_resource::compile_for(
                "assets/app/termian.rc",
                ["termian-native"],
                embed_resource::ParamsIncludeDirs([assets]),
            )
            .manifest_required()
            .expect("compile Termian Windows icon");
            return;
        }
        Ok("macos") => {}
        _ => return,
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let sdk = xcrun(&["--sdk", "macosx", "--show-sdk-path"]);
    let swiftc = PathBuf::from(xcrun(&["--sdk", "macosx", "--find", "swiftc"]));
    let swift_libraries = swiftc
        .parent()
        .expect("Swift bin directory")
        .join("../lib/swift/macosx");
    let arch = env::var("CARGO_CFG_TARGET_ARCH").expect("CARGO_CFG_TARGET_ARCH");
    let arch = if arch == "aarch64" { "arm64" } else { &arch };
    let version = env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| "11.0".into());
    let status = Command::new("xcrun")
        .args([
            "--sdk",
            "macosx",
            "swiftc",
            "-parse-as-library",
            "-O",
            "-emit-library",
            "-static",
            "-target",
            &format!("{arch}-apple-macosx{version}"),
            "-sdk",
            &sdk,
            "-module-name",
            "TermianVault",
            "src/store/macos.swift",
            "-o",
        ])
        .arg(out.join("libtermian_vault.a"))
        .status()
        .expect("run swiftc");
    assert!(status.success(), "macOS vault helper failed to compile");

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-search=native={sdk}/usr/lib/swift");
    println!(
        "cargo:rustc-link-search=native={}",
        swift_libraries.display()
    );
    println!("cargo:rustc-link-lib=static=termian_vault");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
}
