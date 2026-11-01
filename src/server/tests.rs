use std::io;

use super::request::Method;
use super::{Route, passes};

#[test]
fn a_failure_to_accept_for_lack_of_descriptors_memory_or_buffers_or_an_aborted_connection_passes() {
    use rustix::io::Errno;

    for errno in [
        Errno::MFILE,
        Errno::NFILE,
        Errno::CONNABORTED,
        Errno::NOBUFS,
        Errno::NOMEM,
    ] {
        assert!(passes(&io::Error::from(errno)), "{errno:?}");
    }
    assert!(!passes(&io::Error::from(Errno::BADF)));
    assert!(!passes(&io::Error::other("no errno")));
}

#[test]
fn the_run_routes_take_their_ids_from_the_path_and_only_by_post() {
    assert!(matches!(
        Route::of(Method::Post, "/api/confirmations/abc"),
        Route::Confirmation("abc")
    ));
    assert!(matches!(
        Route::of(Method::Post, "/api/runs/3/cancel"),
        Route::Cancel("3")
    ));
    assert!(matches!(Route::of(Method::Post, "/api/runs"), Route::Runs));
    assert!(matches!(
        Route::of(Method::Post, "/api/previews"),
        Route::Previews
    ));
    for path in ["/api/runs", "/api/runs/3/cancel", "/api/confirmations/abc"] {
        assert!(
            matches!(Route::of(Method::Get, path), Route::ApiNotFound),
            "{path}"
        );
    }
}
