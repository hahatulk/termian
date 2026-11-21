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

The web panel listens on `SYNC_PANEL_BIND` (default `127.0.0.1:8081`; `off` disables it). By default it answers only to `127.0.0.1`/`localhost`; `SYNC_PANEL_ALLOWED_HOSTS` adds comma-separated names or IPs (e.g. `192.168.1.5,sync.example.com`) or `*` for any host. Docker Compose binds `0.0.0.0:8081` with `SYNC_PANEL_ALLOWED_HOSTS=*` unless you override it in `.env`. Open <http://127.0.0.1:8081> (or `http://<server>:8081`) and enter the **encryption password** from Termian's «Своё облако» settings. The panel has no password length or character rules. Leave the field blank if the snapshot was encrypted with an empty password. The server decrypts the snapshot in memory to verify it; the password is never stored or logged.

After unlocking the panel shows:

- the last sync (time of the last upload), the last client download since start, the revision hash, snapshot size and limit;
- stored revisions, uptime, currently locked-out client addresses and the lockout policy;
- when the snapshot was created and how many hosts, groups, keys, identities, known hosts, port forwarding rules, snippets and settings it contains. Current clients never sync known hosts, so their count is 0 unless the snapshot came from an older client.

**Выгрузить данные (JSON)** requires the sync token as well as the snapshot password, even for a snapshot with an empty password. The downloaded JSON contains passwords and private keys in plain text.

Protection: requests with a `Host` outside the allow-list are rejected (DNS rebinding; `*` turns this check off), the API accepts only JSON with an `X-Termian-Panel` header (no cross-site form posts), a strict CSP is sent, and panel password failures are counted by TCP peer IP. Export and password rotation share a separate counter for invalid or missing sync tokens. During token lockout both return `429` with `Retry-After`, even for a correct token. Behind a reverse proxy, clients share the proxy IP. The panel accepts one decryption or rotation operation at a time and returns `429` with `Retry-After` while busy. The panel speaks plain HTTP: over an untrusted network the encryption password and token travel unencrypted, so prefer listing exact hosts behind an HTTPS reverse proxy, a VPN, or an SSH tunnel (`ssh -L 8081:127.0.0.1:8081 server`).

### Change the encryption password

Pause cloud sync and log backups on every client before rotating. Open the web panel with the current password, enter the sync token and the new password twice, then confirm. The new password must be non-empty and at most 4096 UTF-8 bytes. Opening an existing snapshot accepts its original password, including an empty password. The server re-encrypts the current snapshot and session logs together. If a log still uses another password, rotation stops without changing the active files.

In each native client, enter the new password in «Своё облако» and choose «Сменить пароль». The client verifies that the new password opens the snapshot before saving it. Future uploads use the new password. Historical revisions and older client `.json` backups still require their original passwords. Current client `.bin` backups use the local Vault key. After switching to the new generation, the server removes the revision files it copied and verified from the preceding generation. Rotation does not erase older inactive generations or external backups. After rotation, keep `.active-storage` and its `sealed-*` storage directory together when backing up the server. Older server binaries cannot read that layout.

### Failed authorization limit

Each client address may send `SYNC_AUTH_MAX_FAILURES` (default `5`) requests with a wrong bearer token. Every further failure locks the address out for `SYNC_AUTH_LOCKOUT_SECONDS` (default `30`), doubling with each failure up to one hour. While locked out the server answers `429 Too Many Requests` with `Retry-After`, **even for the correct token**, so guesses cannot be confirmed. A successful request resets the counter.

Behind a reverse proxy every request comes from the proxy's address. Set `SYNC_TRUST_FORWARDED_FOR=1` only when the proxy overwrites `X-Forwarded-For`; otherwise clients could spoof it.

The sync API cannot detect a wrong **encryption password**: it only stores ciphertext. The Termian client limits repeated failed decryptions locally; the web panel limits them separately.

## Vaults

One server stores all Vaults for a sync account. Enter its address, bearer token and encryption password once in Termian. A normal sync exchanges the encrypted Vault list and merges each Vault separately. The token grants access to every Vault on this server.

"Personal" keeps `/v1/snapshot` and `/v1/logs`. An upgraded server keeps reading those files from the same data volume. Other Vaults have `workspaces/<uuid>/snapshot.bin`, `revisions/` and `logs/` in the same active storage. `workspaces/catalog/snapshot.bin` contains the encrypted Vault IDs and names. The server API does not decrypt the list.

The panel displays and exports "Personal", or the first available Vault when "Personal" has no snapshot. Password rotation covers the catalog, all snapshots and session logs in one storage generation.

## API

Snapshot, Vault list and log endpoints require `Authorization: Bearer <token>`.

When a reverse proxy in front of the server requires HTTP Basic auth, the proxy owns the `Authorization` header. Enable Basic auth in the client's sync settings: it then sends `Authorization: Basic …` for the proxy and the token in `X-Termian-Token: <token>`. The proxy must forward that header. Example for nginx:

```nginx
location / {
    auth_basic           "Termian sync";
    auth_basic_user_file /etc/nginx/termian.htpasswd;
    proxy_pass           http://127.0.0.1:8080;
}
```

| Endpoint | Purpose |
| --- | --- |
| `GET /health` | Public health check |
| `GET /v1/snapshot` | Download snapshot with `ETag`; if absent, returns 404 with `X-Termian-Conditional-Create: v1` |
| `PUT /v1/snapshot` | Replace the Personal snapshot; returns new `ETag` |
| `GET`, `PUT /v1/vaults` | Encrypted Vault list with conditional writes |
| `GET`, `PUT /v1/vaults/<uuid>/snapshot` | Snapshot of one Vault with conditional writes |
| `GET`, `PUT /v1/vaults/<uuid>/logs/<name>` | Session log in one Vault |
| `DELETE /v1/vaults/<uuid>/logs?since=<ms>` | Delete session logs in one Vault |
| `GET /v1/logs/<name>` | Download a session log backup or 404 |
| `PUT /v1/logs/<name>` | Store or replace a session log backup |
| `DELETE /v1/logs?since=<ms>` | Delete logs uploaded at or after a Unix time in milliseconds (`0` or none: all); returns `{"removed": n, "failed": 0}` on success. Partial deletion returns HTTP 500 with both counts |

For conditional writes, `If-Match` checks an existing revision, while `If-None-Match: *` permits creation only if absent. Both merge and forced upload use a conditional write. After password rotation, replacing the snapshot requires its exact current ETag. Older clients that upload unconditionally receive HTTP 412. A failed precondition returns HTTP 412. The Vault list and Vault snapshots always require a precondition for writes. Each snapshot retains its last 10 revisions in its own `revisions/`.

Session logs are encrypted by the client (sealed with the encryption password, like the snapshot) and stored in `logs/`, separately from the snapshot and its revisions. Names may contain only letters, digits, `.`, `_` and `-`. Logs not updated for `SYNC_LOG_RETENTION_DAYS` (default `60`) are deleted at start and then every 6 hours. The snapshot response also includes `X-Termian-Key-Generation`. For `PUT /v1/logs/<name>`, the client first decrypts the current snapshot and sends its `ETag` in `X-Termian-Snapshot-Revision` along with `X-Termian-Key-Generation`. The server returns HTTP 412 if there is no snapshot or the revision is missing or stale. After password rotation it also requires the current generation. Older clients that omit the revision cannot upload logs to this server. For a new store, sync once to create the snapshot, then sync again to upload logs.

## Security

Use HTTPS through a reverse proxy outside a trusted local network. Keep the bearer token secret and back up the data volume. The sync API never receives the encryption password. The web panel uses it in memory when opening, exporting, or re-encrypting data. Clients merge changes from multiple devices; the user resolves conflicting edits of the same record.

See the [Termian client](../README.md).
