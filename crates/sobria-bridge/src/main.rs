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
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();

    let socket = default_socket_path();
    while let Some(req) = read_message(&mut input)? {
        let resp = try_forward_to(&socket, &req).unwrap_or_else(|_| handle_request(req));
        write_message(&mut output, &resp)?;
    }
    Ok(())
}
