//! The test process's SIGINT disposition, which every child that a runner
//! starts inherits.
//!
//! A test process that starts with SIGINT ignored, as a background job of a
//! non-interactive shell does, would pass that disposition to the tools it
//! runs. A tool that ignores SIGINT cannot handle a cancel's SIGINT, so the
//! cancel escalates to SIGTERM. `flightdeck open` avoids this by installing
//! its own handlers before it serves, and the tests do the same here.

use std::sync::Once;
use std::sync::atomic::AtomicBool;

use signal_hook::consts::SIGINT;

/// Installs a SIGINT handler in the test process once. A handled signal
/// resets to its default on exec, so every child starts with SIGINT at its
/// default. The handler runs the default action, so Ctrl-C still stops the
/// test process. A test process that started with SIGINT ignored, as a
/// background job, is then stopped by a SIGINT that it used to ignore.
pub(crate) fn interrupt_reaches_children() {
    static INSTALLED: Once = Once::new();
    INSTALLED.call_once(|| {
        static ALWAYS: AtomicBool = AtomicBool::new(true);
        signal_hook::flag::register_conditional_default(SIGINT, &ALWAYS).unwrap();
    });
}
