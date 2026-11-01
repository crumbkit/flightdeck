use std::io::{Read as _, Write as _};
use std::time::{Duration, Instant};

use super::{DRAIN_GRACE, Drained};
use crate::spawn::stream::queue::queue;

#[test]
fn a_drained_pipe_reads_what_was_written_and_ends_a_grace_after_the_reap_while_its_writer_stays_open()
 {
    let (reader, mut writer) = std::io::pipe().unwrap();
    writer.write_all(b"last words\n").unwrap();
    let (lines, _end, _receiver) = queue(1);
    let mut drained = Drained::new(reader, lines.clone());

    lines.reaped();
    let started = Instant::now();
    let mut read = String::new();
    drained.read_to_string(&mut read).unwrap();

    assert_eq!(read, "last words\n");
    assert!(started.elapsed() >= DRAIN_GRACE - Duration::from_millis(50));
    assert!(started.elapsed() < DRAIN_GRACE + Duration::from_secs(1));
    drop(writer);
}
