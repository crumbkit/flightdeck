use std::io;

use crumbkit::{Failure as _, Recovery};
use signal_hook::consts::{SIGINT, SIGTERM};

use super::{OpenError, forced_exit, printable};
use crate::cli::TOOL;
use crate::server::StartError;

#[test]
fn printable_text_escapes_control_and_bidirectional_characters_and_keeps_the_rest() {
    let text = "shelf 1.0\u{1b}[2J\t\u{7f}\u{200f}\u{2066}é·";

    assert_eq!(
        printable(text),
        "shelf 1.0\\u{1b}[2J\\u{9}\\u{7f}\\u{200f}\\u{2066}é·"
    );
}

#[test]
fn a_server_failure_with_no_repair_of_its_own_recovers_by_running_open_again() {
    let failures = [
        OpenError::Signals(io::Error::other("refused")),
        OpenError::Random(getrandom::Error::UNSUPPORTED),
        OpenError::Serve(io::Error::other("refused")),
    ];

    for failure in failures {
        assert_eq!(failure.recovery(), Recovery::Retry, "{failure:?}");
        assert!(failure.alternatives().is_empty(), "{failure:?}");
    }
}

#[test]
fn a_server_failure_with_a_repair_names_the_check_and_offers_running_open_again() {
    let start = OpenError::Start(StartError::Listen(io::Error::other("refused")));
    let opener = OpenError::Opener(io::Error::other("read-only"));

    assert_eq!(
        start.recovery(),
        Recovery::hint("check that flightdeck may listen on 127.0.0.1")
    );
    assert_eq!(start.alternatives(), [Recovery::Retry]);
    assert_eq!(
        opener.recovery(),
        Recovery::hint("check that the temporary directory is writable")
    );
    assert_eq!(opener.alternatives(), [Recovery::Retry]);
}

#[test]
fn a_relative_path_entry_offers_the_tool_s_path_and_another_directory_as_two_repairs() {
    let failure = OpenError::SearchPath;

    assert_eq!(failure.recovery(), Recovery::hint("give the tool's path"));
    assert_eq!(
        failure.alternatives(),
        [Recovery::hint("run flightdeck open from another directory")]
    );
}

#[test]
fn the_forced_exit_of_each_signal_is_the_shell_s_code_for_it_and_the_tool_declares_it() {
    let declared: Vec<u8> = TOOL
        .get_failures()
        .get_exits()
        .iter()
        .map(crumbkit::tool::Exit::code)
        .collect();

    assert_eq!(forced_exit(SIGINT), 130);
    assert_eq!(forced_exit(SIGTERM), 143);
    for signal in [SIGINT, SIGTERM] {
        let code = u8::try_from(forced_exit(signal)).unwrap();
        assert!(declared.contains(&code), "{code} in {declared:?}");
    }
}
