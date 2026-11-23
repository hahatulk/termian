# Termian sync server

This Rust server stores encrypted Vault snapshots and session logs for [Termian](../README.md).

## Overview

The client encrypts data before upload. One bearer token grants access to all Vaults on the server. The optional web panel can open snapshots and change the encryption password.

## Installation

Install Docker with Compose, `openssl`, and `curl`. For a new installation, run from the repository root:

```bash
cd cloud-sync-server &&
(
  umask 077
  set -C
  token=$(openssl rand -hex 32) &&
    printf 'SYNC_BEARER_TOKEN=%s\n' "$token" > .env
) &&
docker compose up -d --build
```

If `.env` already exists, run `docker compose up -d --build` from `cloud-sync-server/` to reuse it.

Without Docker, install stable Rust. Export a generated `SYNC_BEARER_TOKEN` and run `cargo run --locked --release` from `cloud-sync-server/`. Keep the same token for future starts.

## Usage

```bash
curl -fsS http://127.0.0.1:8080/health
```

The server returns `ok`. In Termian's Settings → Account, enter the server URL, the token from `.env` or the server environment, and an encryption password. Use `http://127.0.0.1:8080` if the client runs on the server. Use the same sync settings on each device.

Open <http://127.0.0.1:8081> on the server to view the web panel. Unlock it with the encryption password. JSON export also requires the bearer token and contains passwords and private keys in plaintext.

## Configuration

- `SYNC_BEARER_TOKEN` is required and must have at least 32 characters. Keep it secret.
- `SYNC_BIND` defaults to `0.0.0.0:8080`. `SYNC_MAX_BODY_BYTES` limits requests to 64 MiB by default.
- `SYNC_DATA_DIR` defaults to `./data`. Compose stores data in the `sync-data` volume. Back up the volume. **`docker compose down -v` deletes it.**
- `SYNC_LOG_RETENTION_DAYS` defaults to `60`.
- `SYNC_AUTH_MAX_FAILURES` defaults to `5`. Further invalid tokens trigger a lockout starting at `SYNC_AUTH_LOCKOUT_SECONDS` (`30`).
- `SYNC_PANEL_BIND` defaults to `127.0.0.1:8081` outside Docker. Use `off` to disable the panel. `SYNC_PANEL_ALLOWED_HOSTS` takes comma-separated hostnames or IP addresses. Compose publishes port `8081` on all interfaces and defaults to allowing any host header. Restrict access before exposing the panel.

Use HTTPS for remote clients. If a proxy requires Basic auth, enable it in the client and forward `X-Termian-Token` to the sync API. Set `SYNC_TRUST_FORWARDED_FOR=1` only if a trusted proxy overwrites `X-Forwarded-For`.

## Contributing

Report bugs in [GitHub Issues](https://github.com/hahatulk/termian/issues) or submit a pull request. Run `cargo test --locked --manifest-path cloud-sync-server/Cargo.toml` from the repository root before submitting server changes.

## License

[MIT](../LICENSE)
