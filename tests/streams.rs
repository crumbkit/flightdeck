//! flightdeck's handling of a standard output that closes before it
//! finishes printing, as under `head` (CLI-8).

#![cfg(test)]
#![expect(
    clippy::disallowed_methods,
    reason = "the test starts the flightdeck binary"
)]

use std::process::{Command, Stdio};

#[test]
fn a_closed_standard_output_stops_a_command_without_a_panic_or_a_message() {
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);

    let output = Command::new(env!("CARGO_BIN_EXE_flightdeck"))
        .args(["commands", "list", "--json", "--full"])
        .stdin(Stdio::null())
        .stdout(writer)
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}
