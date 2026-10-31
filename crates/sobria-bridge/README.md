# sobria-bridge

Native Messaging host between the Sobr.ia browser extension and the Sobr.ia
desktop app.

## What it does

The browser starts the `sobria-bridge` binary and talks to it on stdin and
stdout: each message is a little-endian `u32` length followed by UTF-8 JSON.
The bridge opens no network port and runs only while the browser keeps the
port open.

Each request (`ping`, `pair`, `estimate`, `revoke`) is forwarded to the
desktop app over a local channel, and the app's reply is passed back as is.
When the forward fails, the bridge answers by itself:

- `ping` → `{ ok: true, pong: true }` (the bridge is installed);
- `estimate` → appended to the spool (see below);
- `pair` and `revoke` → refused, they need the app.

Local channel: `$XDG_RUNTIME_DIR/sobria-bridge.sock`, or
`/tmp/sobria-bridge.sock` without it (Unix), and `\\.\pipe\sobria-bridge`
(Windows). Known gaps: the channel is not restricted to the current OS user
yet (SOB-107), the Windows forward has no timeout (SOB-96), and the bridge
falls back even after the app received the request (SOB-190).

## Installation

The desktop app writes the native messaging manifests itself
(`sobria-app`, `bridge_install.rs`).

For development, the scripts build the bridge in release mode and fill
`manifest.template.json` (`{{BRIDGE_PATH}}`, `{{EXTENSION_ID}}`) for Chrome
and Firefox:

```bash
crates/sobria-bridge/scripts/install-dev.sh <EXTENSION_ID>      # macOS, Linux
```

```powershell
crates\sobria-bridge\scripts\install-dev.ps1 -ExtensionId <ID>  # Windows
```

The extension ID is shown in `chrome://extensions` once the extension is
loaded. The manifest written for Firefox is the Chrome one, so the Firefox dev
install does not work yet (SOB-82).

Manifest locations, for a manual install:

| OS | Browser | Location |
| -- | -- | -- |
| macOS | Chrome | `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/com.sobria.bridge.json` |
| macOS | Firefox | `~/Library/Application Support/Mozilla/NativeMessagingHosts/com.sobria.bridge.json` |
| Linux | Chrome | `~/.config/google-chrome/NativeMessagingHosts/com.sobria.bridge.json` |
| Linux | Firefox | `~/.mozilla/native-messaging-hosts/com.sobria.bridge.json` |
| Windows | Chrome | registry key `HKCU\Software\Google\Chrome\NativeMessagingHosts\com.sobria.bridge`, value: path of the JSON file |
| Windows | Firefox | registry key `HKCU\Software\Mozilla\NativeMessagingHosts\com.sobria.bridge`, value: path of the JSON file |

To check, open the extension's service worker console and run:

```js
const port = chrome.runtime.connectNative('com.sobria.bridge');
port.onMessage.addListener((m) => console.log('bridge response:', m));
port.postMessage({ type: 'ping', reqId: 'manual-1' });
// → { reqId: 'manual-1', ok: true, pong: true }
```

If the port disconnects at once, check the absolute path in the manifest, that
the binary is executable, and the extension ID in `allowed_origins`.

## Spool

Offline estimates go to `~/.sobria/spool/incoming.jsonl`, one JSON line each:
`{ secret_hash, payload, received_at }`. The app drains it
(`sobria-app`, `extension_store.rs`). Above 10 MB the file is moved to
`incoming.jsonl.bak` and a new one starts.

The secret is not checked on this path (SOB-119), and `secret_hash` is a weak,
unkeyed hash (SOB-81).

## Tests

```bash
cargo test -p sobria-bridge
```

`tests/bridge_process.rs` and `tests/forward_wire.rs` run on Unix only (they
fake the app on a Unix socket and isolate `HOME`).
