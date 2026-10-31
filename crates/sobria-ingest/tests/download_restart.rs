//! Characterization tests of the downloader's retry and of the timeout it relies
//! on (SOB-222).
//!
//! The ComparIA files weigh 682 MB, 733 MB and 3.4 GB. The audit found that
//! the timeout of `Downloader::new()` (`DEFAULT_TIMEOUT`, 60 s) covers the whole
//! body and that a retry of `fetch_to_file` starts again at byte 0. A
//! hand-written local server plays a connection that stalls and one that drips.
//!
//! Both tests build their own client with a short timeout, because the 60 s of
//! `new()` would make them last a minute. The first one pins `fetch_to_file`
//! itself (retry from byte 0, no Range header). The second one pins the
//! `timeout()` setting of reqwest that `new()` applies, not `new()`: when
//! SOB-123 changes the timeout of `new()`, it adds a test that goes through it.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
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
    /// Body bytes written so far, by connection number (0 is the first).
    dripped: Arc<Mutex<HashMap<usize, usize>>>,
}

/// Starts a server where connection `i` follows `scripts[i]` (the last script
/// repeats once the list is used up).
async fn start(scripts: Vec<Script>) -> Stub {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let url = format!("http://{}/file", listener.local_addr().expect("address"));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let dripped = Arc::new(Mutex::new(HashMap::new()));
    let (seen, sent) = (Arc::clone(&requests), Arc::clone(&dripped));
    let last = *scripts.last().expect("a script");
    let mut scripts = scripts.into_iter().chain(std::iter::repeat(last));

    tokio::spawn(async move {
        let mut connection = 0;
        while let Ok((stream, _)) = listener.accept().await {
            let script = scripts.next().expect("an endless iterator");
            tokio::spawn(serve(
                stream,
                script,
                connection,
                Arc::clone(&seen),
                Arc::clone(&sent),
            ));
            connection += 1;
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
    connection: usize,
    seen: Arc<Mutex<Vec<String>>>,
    dripped: Arc<Mutex<HashMap<usize, usize>>>,
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
                *dripped.lock().expect("lock").entry(connection).or_default() += 1;
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
async fn download_quirk_the_reqwest_timeout_covers_the_whole_body_even_while_data_flows() {
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
    // Bytes were arriving on the first connection when the client gave up.
    let first_connection = stub.dripped.lock().expect("lock").get(&0).copied();
    assert!(first_connection >= Some(2), "{first_connection:?}");
    assert_eq!(
        stub.requests.lock().expect("lock").len(),
        2,
        "the first attempt and one retry"
    );
    assert!(!dest.exists());
    assert!(!tmp.path().join("file.bin.partial").exists());
}
