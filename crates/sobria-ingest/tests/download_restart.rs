//! Characterization tests of the downloader's timeout and retry (SOB-222).
//!
//! The ComparIA files weigh 682 MB, 733 MB and 3.4 GB. The audit found that
//! the 60 s timeout (`DEFAULT_TIMEOUT`) covers the whole body and that a retry
//! of `fetch_to_file` starts again at byte 0. A hand-written local server plays
//! a connection that stalls and one that drips, with a short timeout on the
//! client. SOB-123 is the issue that fixes it.

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use sobria_ingest::{DownloadStatus, Downloader, IngestError};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

const BODY: &[u8] = b"0123456789";

/// What the local server does with one connection.
#[derive(Clone, Copy)]
enum Script {
    /// Sends the headers and the first half of the body, then goes silent.
    StallAfterHalf,
    /// Sends everything at once.
    Full,
    /// Sends one byte every 200 ms: data keeps flowing, slowly.
    Drip,
}

struct Stub {
    url: String,
    /// The raw text of each request, lower-cased, in arrival order.
    requests: Arc<Mutex<Vec<String>>>,
    /// Body bytes written by the [`Script::Drip`] connections.
    dripped: Arc<AtomicUsize>,
}

/// Starts a server where connection `i` follows `scripts[i]` (the last script
/// repeats once the list is used up).
async fn start(scripts: Vec<Script>) -> Stub {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let url = format!("http://{}/file", listener.local_addr().expect("address"));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let dripped = Arc::new(AtomicUsize::new(0));
    let (seen, sent) = (Arc::clone(&requests), Arc::clone(&dripped));
    let last = *scripts.last().expect("a script");
    let mut scripts = scripts.into_iter().chain(std::iter::repeat(last));

    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let script = scripts.next().expect("an endless iterator");
            tokio::spawn(serve(stream, script, Arc::clone(&seen), Arc::clone(&sent)));
        }
    });
    Stub {
        url,
        requests,
        dripped,
    }
}

async fn serve(
    mut stream: TcpStream,
    script: Script,
    seen: Arc<Mutex<Vec<String>>>,
    dripped: Arc<AtomicUsize>,
) {
    let mut request = Vec::new();
    let mut chunk = [0u8; 1024];
    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut chunk).await {
            Ok(0) | Err(_) => return,
            Ok(n) => request.extend_from_slice(&chunk[..n]),
        }
    }
    seen.lock()
        .expect("lock")
        .push(String::from_utf8_lossy(&request).to_lowercase());

    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        BODY.len()
    );
    let _ = stream.write_all(head.as_bytes()).await;
    match script {
        Script::StallAfterHalf => {
            let _ = stream.write_all(&BODY[..BODY.len() / 2]).await;
            tokio::time::sleep(Duration::from_secs(60)).await;
        },
        Script::Full => {
            let _ = stream.write_all(BODY).await;
        },
        Script::Drip => {
            for byte in BODY {
                if stream.write_all(&[*byte]).await.is_err() {
                    return;
                }
                dripped.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        },
    }
}

fn downloader(timeout: Duration, retries: u32) -> Downloader {
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(timeout)
        .build()
        .expect("client");
    Downloader::with_client(client).with_max_retries(retries)
}

#[tokio::test]
async fn download_quirk_a_timeout_restarts_the_file_from_byte_zero_without_a_range_header() {
    let stub = start(vec![Script::StallAfterHalf, Script::Full]).await;
    let tmp = tempfile::tempdir().expect("tmp");
    let dest = tmp.path().join("file.bin");

    let outcome = downloader(Duration::from_millis(700), 2)
        .fetch_to_file(&stub.url, &dest, None)
        .await
        .expect("the second attempt succeeds");

    assert_eq!(outcome.status, DownloadStatus::Downloaded);
    assert_eq!(outcome.bytes, 10);
    assert_eq!(std::fs::read(&dest).expect("file"), BODY);
    assert!(!tmp.path().join("file.bin.partial").exists());

    let requests = stub.requests.lock().expect("lock");
    assert_eq!(requests.len(), 2, "one request per attempt");
    for request in requests.iter() {
        assert!(
            !request.contains("\r\nrange:"),
            "no Range header, so no resume: {request}"
        );
    }
}

#[tokio::test]
async fn download_quirk_the_timeout_covers_the_whole_body_even_while_data_flows() {
    // 10 bytes at 200 ms each take 2 s; the client gives up after 0.5 s although
    // a byte arrives every 200 ms. A timeout on idle time would let it finish.
    let stub = start(vec![Script::Drip]).await;
    let tmp = tempfile::tempdir().expect("tmp");
    let dest = tmp.path().join("file.bin");

    let error = downloader(Duration::from_millis(500), 1)
        .fetch_to_file(&stub.url, &dest, None)
        .await
        .expect_err("the download should time out");

    assert!(
        matches!(&error, IngestError::Http(e) if e.is_timeout()),
        "{error:?}"
    );
    assert!(
        stub.dripped.load(Ordering::SeqCst) >= 2,
        "bytes were arriving when the client gave up"
    );
    assert_eq!(
        stub.requests.lock().expect("lock").len(),
        2,
        "the first attempt and one retry"
    );
    assert!(!dest.exists());
    assert!(!tmp.path().join("file.bin.partial").exists());
}
