# Self-hosted Sync Server

A small Rust server for Termian's encrypted snapshots. Encryption happens in the client; the sync API stores opaque bytes. Only the optional local web panel decrypts a snapshot in memory, with the password you enter.

## Docker

From `cloud-sync-server/`:

```bash
cp .env.example .env
# Set SYNC_BEARER_TOKEN in .env to the output of: openssl rand -hex 32
docker compose up -d --build
curl -fsS http://127.0.0.1:8080/health
```

Compose uses the persistent `sync-data` volume. **Do not run `docker compose down -v`** unless you intend to delete stored snapshots.

## Run without Docker

```bash
export SYNC_BEARER_TOKEN="$(openssl rand -hex 32)"
export SYNC_DATA_DIR=./data
cargo run --release
```

The default bind address is `0.0.0.0:8080` (`SYNC_BIND`); the maximum request body is 64 MiB (`SYNC_MAX_BODY_BYTES`).

### Web panel

A local panel listens on `SYNC_PANEL_BIND` (default `127.0.0.1:8081`; `off` disables it). Docker Compose publishes it on the host's `127.0.0.1:8081` only. Open <http://127.0.0.1:8081> and enter the **encryption password** from Termian's «Своё облако» settings. The server decrypts the snapshot in memory to verify it; the password is never stored or logged.

After unlocking the panel shows:

- the last sync (time of the last upload), the last client download since start, the revision hash, snapshot size and limit;
- stored revisions, uptime, currently locked-out client addresses and the lockout policy;
- when the snapshot was created and how many hosts, groups, keys, identities, known hosts, port forwarding rules, snippets and settings it contains. Current clients never sync known hosts, so their count is 0 unless the snapshot came from an older client.

**Выгрузить данные (JSON)** downloads the decrypted snapshot — the same JSON the client imports. It contains passwords and private keys in plain text.

Protection: requests with a non-loopback `Host` are rejected (DNS rebinding), the API accepts only JSON with an `X-Termian-Panel` header (no cross-site form posts), a strict CSP is sent, and wrong passwords use the same lockout as wrong tokens. For remote access use an SSH tunnel: `ssh -L 8081:127.0.0.1:8081 server`.

### Failed authorization limit

Each client address may send `SYNC_AUTH_MAX_FAILURES` (default `5`) requests with a wrong bearer token. Every further failure locks the address out for `SYNC_AUTH_LOCKOUT_SECONDS` (default `30`), doubling with each failure up to one hour. While locked out the server answers `429 Too Many Requests` with `Retry-After`, **even for the correct token**, so guesses cannot be confirmed. A successful request resets the counter.

Behind a reverse proxy every request comes from the proxy's address. Set `SYNC_TRUST_FORWARDED_FOR=1` only when the proxy overwrites `X-Forwarded-For`; otherwise clients could spoof it.

The sync API cannot detect a wrong **encryption password**: it only stores ciphertext. The Termian client limits repeated failed decryptions locally; the web panel limits them separately.

## API

Snapshot endpoints require `Authorization: Bearer <token>`.

| Endpoint | Purpose |
| --- | --- |
| `GET /health` | Public health check |
| `GET /v1/snapshot` | Download snapshot; returns `ETag` or 404 |
| `PUT /v1/snapshot` | Replace snapshot; returns new `ETag` |
| `GET /v1/logs/<name>` | Download a session log backup or 404 |
| `PUT /v1/logs/<name>` | Store or replace a session log backup |
| `DELETE /v1/logs?since=<ms>` | Delete logs uploaded at or after a Unix time in milliseconds (`0` or none: all); returns `{"removed": n}` |

For conditional writes, `If-Match` checks an existing revision, while `If-None-Match: *` permits creation only if absent. Clients merging changes from several devices always upload with one of them, so a snapshot uploaded meanwhile by another device is never overwritten. A failed precondition returns HTTP 412. The last 10 revisions are retained in `revisions/`.

Session logs are encrypted by the client (sealed with the encryption password, like the snapshot) and stored in `logs/`, separately from the snapshot and its revisions. Names may contain only letters, digits, `.`, `_` and `-`. Logs not updated for `SYNC_LOG_RETENTION_DAYS` (default `60`) are deleted at start and then every 6 hours. Older servers without these routes answer 404, and clients then skip log backups.

## Security

Use HTTPS through a reverse proxy outside a trusted local network. Keep the bearer token secret and back up the data volume. The **encryption password belongs only in the client** — never on the server. Clients merge changes of several devices themselves (the server only stores ciphertext); conflicting edits of the same record are resolved by the user.

See the [Termian client](../README.md).
