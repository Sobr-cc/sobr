//! Pins the transport-failure path of the three ODRÉ fetch functions (SOB-189).
//!
//! The ODRÉ base URL is a constant over HTTPS, so a stub server cannot answer in its place.
//! The requests go through a local proxy that refuses every tunnel: each function must fail
//! with its own message, and the proxy sees that the tunnel is asked for the ODRÉ host.
//! One test only: the proxy variables are process-wide.

use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use sobria_ingest::sources::territoire_fr::{discover_datasets, fetch_industrial_sites, fetch_rte_mix};

#[tokio::test]
async fn each_fetch_fails_with_its_own_message_when_the_tunnel_is_refused() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let proxy = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    let asked = Arc::new(Mutex::new(Vec::<String>::new()));
    let asked_by_proxy = Arc::clone(&asked);
    let proxy_thread = thread::spawn(move || {
        for stream in listener.incoming().take(3) {
            let mut stream = stream.unwrap();
            let mut first_line = String::new();
            BufReader::new(stream.try_clone().unwrap())
                .read_line(&mut first_line)
                .unwrap();
            asked_by_proxy
                .lock()
                .unwrap()
                .push(first_line.trim_end().to_string());
            stream
                .write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .unwrap();
        }
    });
    for name in ["HTTPS_PROXY", "https_proxy"] {
        std::env::set_var(name, &proxy);
    }
    for name in ["ALL_PROXY", "all_proxy", "NO_PROXY", "no_proxy"] {
        std::env::remove_var(name);
    }
    let limit = Duration::from_secs(30);

    let catalog = tokio::time::timeout(limit, discover_datasets("industrie", 5))
        .await
        .unwrap()
        .unwrap_err();
    let sites = tokio::time::timeout(limit, fetch_industrial_sites(10))
        .await
        .unwrap()
        .unwrap_err();
    let mix = tokio::time::timeout(limit, fetch_rte_mix(2023))
        .await
        .unwrap()
        .unwrap_err();
    proxy_thread.join().unwrap();

    assert_eq!(catalog.to_string(), "requête catalogue ODRÉ échouée");
    assert_eq!(sites.to_string(), "requête ODRÉ IRIS échouée");
    assert_eq!(mix.to_string(), "requête RTE eco2mix échouée");
    assert_eq!(
        *asked.lock().unwrap(),
        ["CONNECT odre.opendatasoft.com:443 HTTP/1.1"; 3]
    );
}
