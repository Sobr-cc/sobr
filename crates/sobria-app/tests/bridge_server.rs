//! Tests of the public `bridge_server` API of the app (patch C27 v0.6.0).
//!
//! - `ping_roundtrip_via_unix_socket` (Unix only) does not start the real
//!   server: it runs its own accept loop on a temp socket and calls
//!   `dispatch_request`. The `SOBRIA_BRIDGE_SOCKET` variable it sets is read by
//!   nothing: on Unix `default_socket_path` only looks at `XDG_RUNTIME_DIR`,
//!   and on Windows it is a fixed pipe name.
//! - There is no Windows test in this file; on Windows only
//!   `dispatch_ping_returns_pong_via_public_api` runs.
//! - The real server, through a real socket or pipe, is tested in
//!   `src/bridge_socket_tests.rs` (SOB-188).

use std::sync::Arc;

use sobria_app::AppState;
use sobria_bridge::BridgeRequest;

#[cfg(unix)]
mod unix {
    use super::*;
    use sobria_bridge::BridgeResponse;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    async fn write_request<S>(stream: &mut S, req: &BridgeRequest)
    where
        S: AsyncWriteExt + Unpin,
    {
        let bytes = serde_json::to_vec(req).unwrap();
        let len = u32::try_from(bytes.len()).unwrap();
        stream.write_all(&len.to_le_bytes()).await.unwrap();
        stream.write_all(&bytes).await.unwrap();
        stream.flush().await.unwrap();
    }

    async fn read_response<S>(stream: &mut S) -> BridgeResponse
    where
        S: AsyncReadExt + Unpin,
    {
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.unwrap();
        let len = u32::from_le_bytes(len_buf) as usize;
        let mut payload = vec![0u8; len];
        stream.read_exact(&mut payload).await.unwrap();
        serde_json::from_slice(&payload).unwrap()
    }

    #[tokio::test]
    async fn ping_roundtrip_via_unix_socket() {
        let tmp = tempfile::tempdir().unwrap();
        let state = Arc::new(AppState::init_in(tmp.path()).unwrap());
        let socket_path = tmp.path().join("sobria-bridge-test.sock");

        // Sets SOBRIA_BRIDGE_SOCKET, which nothing reads today (see the module
        // doc); the test binds the socket directly.
        std::env::set_var("SOBRIA_BRIDGE_SOCKET", &socket_path);

        let server_state = Arc::clone(&state);
        let path_clone = socket_path.clone();
        let server_handle = tokio::spawn(async move {
            // Its own accept loop, not `bridge_server::serve` (see SOB-202).
            let _ = std::fs::remove_file(&path_clone);
            let listener = tokio::net::UnixListener::bind(&path_clone).unwrap();
            let (mut stream, _) = listener.accept().await.unwrap();
            // Reads the request, dispatches it, writes the response.
            let mut len_buf = [0u8; 4];
            stream.read_exact(&mut len_buf).await.unwrap();
            let len = u32::from_le_bytes(len_buf) as usize;
            let mut payload = vec![0u8; len];
            stream.read_exact(&mut payload).await.unwrap();
            let req: BridgeRequest = serde_json::from_slice(&payload).unwrap();
            let resp = sobria_app::bridge_server::dispatch_request(&server_state, req);
            let bytes = serde_json::to_vec(&resp).unwrap();
            let resp_len = u32::try_from(bytes.len()).unwrap();
            stream.write_all(&resp_len.to_le_bytes()).await.unwrap();
            stream.write_all(&bytes).await.unwrap();
            stream.flush().await.unwrap();
        });

        // Short delay so the listener binds before the client connects.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        write_request(
            &mut stream,
            &BridgeRequest::Ping {
                req_id: "p1".into(),
            },
        )
        .await;
        let resp = read_response(&mut stream).await;
        assert_eq!(resp.req_id, "p1");
        assert!(resp.ok);
        assert_eq!(resp.pong, Some(true));

        server_handle.await.unwrap();
    }
}

/// Cross-platform: dispatch_request can handle a Ping without I/O.
/// Slight overlap with the module's unit tests, useful here to check
/// that the bridge_server module is publicly re-exported.
#[test]
fn dispatch_ping_returns_pong_via_public_api() {
    let tmp = tempfile::tempdir().unwrap();
    let state = Arc::new(AppState::init_in(tmp.path()).unwrap());
    let resp = sobria_app::bridge_server::dispatch_request(
        &state,
        BridgeRequest::Ping {
            req_id: "p1".into(),
        },
    );
    assert!(resp.ok);
    assert_eq!(resp.pong, Some(true));
}
