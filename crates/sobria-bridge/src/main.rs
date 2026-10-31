//! `sobria-bridge` binary: the Native Messaging host started by the browser.
//!
//! Reads framed requests on stdin until the end of the stream. Each one is
//! forwarded to the desktop app with [`try_forward_to`]; on any forward error
//! it is handled offline by [`handle_request`] (spool for estimates). A request
//! that cannot be read ends the process (SOB-190).
#![deny(unsafe_code)]

use anyhow::Result;
use sobria_bridge::{
    default_socket_path, handle_request, read_message, try_forward_to, write_message,
};

fn main() -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();

    let socket = default_socket_path();
    while let Some(req) = read_message(&mut input)? {
        let resp = match try_forward_to(&socket, &req) {
            Ok(r) => r,
            Err(_) => handle_request(req),
        };
        write_message(&mut output, &resp)?;
    }
    Ok(())
}
