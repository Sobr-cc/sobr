//! Aides partagées par les tests de caractérisation de `sobria-bridge` (SOB-11).
//!
//! Chaque fichier de `tests/` est un crate de test distinct : toute fonction
//! qu'il n'utilise pas serait signalée, d'où l'`allow(dead_code)`.
#![allow(dead_code)]

use serde_json::Value;

/// Réponse de `handle_request` à un `Pair` quand l'app desktop n'est pas joignable.
pub const PAIR_OFFLINE_REFUSAL: &str =
    "Pairing nécessite l'app Sobr.ia desktop ≥ 0.6.0 lancée — non disponible (C27.5.b/c).";

/// Réponse de `handle_request` à un `Revoke` quand l'app desktop n'est pas joignable.
pub const REVOKE_OFFLINE_REFUSAL: &str =
    "Revoke nécessite l'app Sobr.ia desktop joignable — effacement local côté extension uniquement.";

/// Taille au-delà de laquelle un message est refusé (1 MiB), dans les deux sens.
pub const MAX_MESSAGE_BYTES: u32 = 1024 * 1024;

/// Trame Native Messaging : longueur `u32` little-endian suivie de `body`.
pub fn frame(body: &[u8]) -> Vec<u8> {
    let len = u32::try_from(body.len()).expect("trame < 4 GiB");
    let mut out = len.to_le_bytes().to_vec();
    out.extend_from_slice(body);
    out
}

/// Trame contenant la sérialisation JSON de `value`.
pub fn json_frame(value: &Value) -> Vec<u8> {
    frame(&serde_json::to_vec(value).expect("JSON sérialisable"))
}

/// Décode toutes les trames d'un flux émis par le bridge.
pub fn parse_frames(mut bytes: &[u8]) -> Vec<Value> {
    let mut out = Vec::new();
    while !bytes.is_empty() {
        assert!(bytes.len() >= 4, "préfixe de longueur tronqué");
        let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        assert!(bytes.len() >= 4 + len, "trame tronquée");
        out.push(serde_json::from_slice(&bytes[4..4 + len]).expect("trame JSON"));
        bytes = &bytes[4 + len..];
    }
    out
}

/// Chaîne de contextes d'une erreur `anyhow` ; panique si `result` a réussi.
pub fn err_text<T: std::fmt::Debug>(result: anyhow::Result<T>) -> String {
    match result {
        Err(e) => format!("{e:#}"),
        Ok(value) => panic!("succès inattendu, échec attendu : {value:?}"),
    }
}

/// Faux serveur de l'app desktop, sur socket Unix. Sert aux tests du forward
/// (`forward_wire.rs`) et du binaire (`bridge_process.rs`).
#[cfg(unix)]
pub mod fake_app {
    use std::io::{Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::sync::mpsc::Receiver;
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    use serde_json::Value;

    /// Ce que le faux serveur fait d'une connexion, une fois la requête lue.
    pub enum Reply {
        /// Répond par une trame JSON valide.
        Json(Value),
        /// Écrit ces octets tels quels (trame volontairement invalide).
        Raw(Vec<u8>),
        /// Ferme sans répondre.
        Close,
        /// Ne répond pas tant que l'émetteur associé n'est pas lâché, et au plus
        /// 10 s : un client sans délai de lecture échoue alors au lieu de bloquer.
        Hold(Receiver<()>),
    }

    /// Accepte une connexion, en attendant au plus `wait` : le faux serveur ne
    /// doit jamais bloquer la CI si le bridge ne se connecte pas.
    fn accept_within(listener: &UnixListener, wait: Duration) -> UnixStream {
        let deadline = Instant::now() + wait;
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(5));
                },
                Err(e) => panic!("le bridge ne s'est pas connecté : {e}"),
            }
        };
        // Sur BSD/macOS le flux accepté hérite du mode non bloquant.
        stream.set_nonblocking(false).expect("flux bloquant");
        stream
    }

    /// Lit une requête length-prefixed et la décode.
    fn read_request(stream: &mut UnixStream) -> Value {
        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .expect("longueur de la requête");
        let mut body = vec![0u8; u32::from_le_bytes(len_buf) as usize];
        stream.read_exact(&mut body).expect("corps de la requête");
        serde_json::from_slice(&body).expect("requête JSON")
    }

    /// Accepte une connexion par élément de `replies` (10 s d'attente au plus
    /// chacune), lit une requête, puis applique la réponse. Le `JoinHandle`
    /// rend les requêtes reçues, dans l'ordre.
    pub fn serve(listener: UnixListener, replies: Vec<Reply>) -> JoinHandle<Vec<Value>> {
        listener
            .set_nonblocking(true)
            .expect("listener non bloquant");
        std::thread::spawn(move || {
            let mut requests = Vec::new();
            for reply in replies {
                let mut stream = accept_within(&listener, Duration::from_secs(10));
                requests.push(read_request(&mut stream));
                match reply {
                    Reply::Json(value) => stream
                        .write_all(&super::json_frame(&value))
                        .expect("écriture de la réponse"),
                    Reply::Raw(bytes) => stream.write_all(&bytes).expect("écriture brute"),
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
