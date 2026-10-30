//! Helpers shared by the `sobria-bridge` characterization tests (SOB-11).
//!
//! Each file in `tests/` is a separate crate, so helpers it does not use would
//! be flagged as dead code.
#![allow(dead_code)]

use serde_json::Value;

/// `handle_request` reply to `Pair` when the desktop app is unreachable.
pub const PAIR_OFFLINE_REFUSAL: &str =
    "Pairing nécessite l'app Sobr.ia desktop ≥ 0.6.0 lancée — non disponible (C27.5.b/c).";

/// `handle_request` reply to `Revoke` when the desktop app is unreachable.
pub const REVOKE_OFFLINE_REFUSAL: &str =
    "Revoke nécessite l'app Sobr.ia desktop joignable — effacement local côté extension uniquement.";

/// Messages above this size (1 MiB) are refused, in both directions.
pub const MAX_MESSAGE_BYTES: u32 = 1024 * 1024;

/// Native Messaging frame: little-endian `u32` length, then `body`.
pub fn frame(body: &[u8]) -> Vec<u8> {
    let len = u32::try_from(body.len()).expect("frame < 4 GiB");
    let mut out = len.to_le_bytes().to_vec();
    out.extend_from_slice(body);
    out
}

pub fn json_frame(value: &Value) -> Vec<u8> {
    frame(&serde_json::to_vec(value).expect("serializable JSON"))
}

pub fn parse_frames(mut bytes: &[u8]) -> Vec<Value> {
    let mut out = Vec::new();
    while !bytes.is_empty() {
        assert!(bytes.len() >= 4, "truncated length prefix");
        let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        assert!(bytes.len() >= 4 + len, "truncated frame");
        out.push(serde_json::from_slice(&bytes[4..4 + len]).expect("JSON frame"));
        bytes = &bytes[4 + len..];
    }
    out
}

/// Full context chain of an `anyhow` error; panics if `result` is `Ok`.
pub fn err_text<T: std::fmt::Debug>(result: anyhow::Result<T>) -> String {
    match result {
        Err(e) => format!("{e:#}"),
        Ok(value) => panic!("expected failure, got success: {value:?}"),
    }
}

/// Fake desktop app server on a Unix socket.
#[cfg(unix)]
pub mod fake_app {
    use std::io::{Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::sync::mpsc::Receiver;
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    use serde_json::Value;

    /// What the fake server does once it has read the request.
    pub enum Reply {
        Json(Value),
        /// Writes these bytes as is (deliberately invalid frame).
        Raw(Vec<u8>),
        /// Closes without replying.
        Close,
        /// Stays silent until the sender is dropped, for 10 s at most, so a
        /// client without a read timeout fails instead of hanging.
        Hold(Receiver<()>),
    }

    /// Bounded wait so the fake server never hangs CI if the bridge does not connect.
    fn accept_within(listener: &UnixListener, wait: Duration) -> UnixStream {
        let deadline = Instant::now() + wait;
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                // EINTR happens on busy runners and under tarpaulin: retry.
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                    ) && Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(5));
                },
                Err(e) => panic!("the bridge did not connect: {e}"),
            }
        };
        // On BSD/macOS the accepted stream inherits non-blocking mode.
        stream.set_nonblocking(false).expect("blocking stream");
        stream
    }

    fn read_request(stream: &mut UnixStream) -> Value {
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).expect("request length");
        let mut body = vec![0u8; u32::from_le_bytes(len_buf) as usize];
        stream.read_exact(&mut body).expect("request body");
        serde_json::from_slice(&body).expect("JSON request")
    }

    /// Accepts one connection per reply (10 s wait each at most) and applies
    /// it. The handle yields the received requests, in order.
    pub fn serve(listener: UnixListener, replies: Vec<Reply>) -> JoinHandle<Vec<Value>> {
        listener
            .set_nonblocking(true)
            .expect("non-blocking listener");
        std::thread::spawn(move || {
            let mut requests = Vec::new();
            for reply in replies {
                let mut stream = accept_within(&listener, Duration::from_secs(10));
                requests.push(read_request(&mut stream));
                match reply {
                    Reply::Json(value) => stream
                        .write_all(&super::json_frame(&value))
                        .expect("write reply"),
                    Reply::Raw(bytes) => stream.write_all(&bytes).expect("write raw bytes"),
                    Reply::Close => {},
                    Reply::Hold(release) => {
                        let _ = release.recv_timeout(Duration::from_secs(10));
                    },
                }
            }
            requests
        })
    }
}
