# Termian

Termian is a Rust client for SSH sessions and SFTP transfers on macOS, Windows, and Android.

## Overview

- Split terminal tabs, saved snippets, and per-host startup commands.
- Local, remote, and dynamic port forwarding.
- Encrypted Vaults for hosts and credentials, encrypted session logs with bookmarks, and optional [self-hosted sync](cloud-sync-server/README.md).

## Installation

Download a package from [GitHub Releases](https://github.com/hahatulk/termian/releases):

- macOS Apple Silicon: `.dmg`.
- Windows x64: `-setup.exe` or the portable `.exe`.
- Android 8 or later: `.apk`.

### Desktop from source

Install stable Rust. On macOS, install the Xcode Command Line Tools and run `export MACOSX_DEPLOYMENT_TARGET=11.0`. On Windows, install MSVC and the Windows SDK.

```bash
git clone https://github.com/hahatulk/termian.git
cd termian
cargo run --locked --bin termian-native
```

### Android from source

On macOS or x86_64 Linux, install JDK 21, Python 3, `zip`, the Android SDK command-line tools, and stable Rust through `rustup`. Set `ANDROID_SDK_ROOT` to your SDK directory, then run from the repository root:

```bash
sdkmanager --install 'platforms;android-37.0' 'build-tools;37.0.0' 'ndk;28.2.13676358'
rustup target add aarch64-linux-android x86_64-linux-android
./scripts/build-android.sh
```

The debug APK is `dist/android/termian-debug.apk`.

## Usage

In Vaults, open Hosts and add a host. Enter an SSH server address, a username, and a password or key. On Android, tap "Save". Select the host to connect, then run `pwd` in the terminal.

To run `pwd` after login, enter it in the host's "Startup Command" field under SSH. Middle-click a desktop terminal tab to close it and disconnect its sessions. Release the mouse button to copy selected terminal text. The desktop client shows "Copied!" in the upper-right corner.

## Configuration

Set the server address, access token, and encryption password in Settings → Account to sync your Vaults through a [server you host](cloud-sync-server/README.md). From Settings → Terminal, choose the theme and text size. Those preferences apply to all Vaults on the device.

## Screenshots

<table>
  <tr>
    <td align="center"><a href="https://github.com/user-attachments/assets/4d2edd91-a4ea-47c8-a944-ab96992bca8e"><img src="https://github.com/user-attachments/assets/4d2edd91-a4ea-47c8-a944-ab96992bca8e" alt="Terminal" width="400"></a><br>Terminal</td>
    <td align="center"><a href="https://github.com/user-attachments/assets/bf0001af-9abf-4fce-95f3-3d3bd9c47117"><img src="https://github.com/user-attachments/assets/bf0001af-9abf-4fce-95f3-3d3bd9c47117" alt="Vault and host editor" width="400"></a><br>Vault and host editor</td>
  </tr>
  <tr>
    <td align="center"><a href="https://github.com/user-attachments/assets/c30cb794-2966-485e-a2a1-67dbfa10b30c"><img src="https://github.com/user-attachments/assets/c30cb794-2966-485e-a2a1-67dbfa10b30c" alt="Sync settings" width="400"></a><br>Sync settings</td>
    <td align="center"><a href="https://github.com/user-attachments/assets/835bfa61-1e41-4c76-b017-0724b1a12ee9"><img src="https://github.com/user-attachments/assets/835bfa61-1e41-4c76-b017-0724b1a12ee9" alt="Server panel" width="300"></a><br>Server panel</td>
  </tr>
  <tr>
    <td align="center" colspan="2"><a href="https://github.com/user-attachments/assets/de8ee1d1-95ec-4f90-a13a-d6150da6936f"><img src="https://github.com/user-attachments/assets/de8ee1d1-95ec-4f90-a13a-d6150da6936f" alt="Update channels" width="400"></a><br>Update channels</td>
  </tr>
</table>

## Contributing

Report bugs in [GitHub Issues](https://github.com/hahatulk/termian/issues) or submit a pull request. Run `cargo test --locked` from the repository root before submitting code changes.

## License

[MIT](LICENSE)
