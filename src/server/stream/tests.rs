use std::time::{Duration, UNIX_EPOCH};

use super::utc_seconds;

#[test]
fn the_epoch_writes_as_the_first_second_of_1970() {
    assert_eq!(utc_seconds(UNIX_EPOCH), "1970-01-01T00:00:00Z");
}

#[test]
fn a_time_with_nanoseconds_writes_its_whole_seconds() {
    let time = UNIX_EPOCH + Duration::new(1_794_305_685, 999_999_999);

    assert_eq!(utc_seconds(time), "2026-11-10T10:14:45Z");
}

#[test]
fn a_time_before_1970_writes_as_the_epoch() {
    let time = UNIX_EPOCH - Duration::from_secs(86_400);

    assert_eq!(utc_seconds(time), "1970-01-01T00:00:00Z");
}

#[test]
fn a_time_after_jiffs_last_timestamp_writes_as_that_timestamp() {
    // 10000-01-01T00:00:00Z, past jiff's last timestamp and inside a Windows clock's range.
    let time = UNIX_EPOCH + Duration::from_secs(253_402_300_800);

    assert_eq!(utc_seconds(time), "9999-12-30T22:00:00Z");
}
