use super::queue;
use crate::spawn::stream::{Ended, Event};
use crate::spawn::{Exit, SpawnError};

fn ended() -> Result<Ended, SpawnError> {
    Ok(Ended {
        exit: Exit::Code(0),
        stopped: None,
        stderr: Vec::new(),
    })
}

/// Returns the line of `event`, or the lost line of an end.
fn text(event: Option<Event>) -> Option<String> {
    match event {
        Some(Event::Line(line)) => Some(format!("line {line}")),
        Some(Event::End { lost, .. }) => Some(format!("end {lost:?}")),
        None => None,
    }
}

#[test]
fn a_full_queue_drops_a_line_once_a_stop_is_marked_and_the_end_carries_it_as_the_last_line() {
    let (lines, end, receiver) = queue(2);
    assert!(lines.push("a".to_owned()));
    assert!(lines.push("b".to_owned()));

    assert!(lines.stop());
    assert!(lines.push("c".to_owned()));
    end.send(ended());

    assert_eq!(text(receiver.recv()).as_deref(), Some("line a"));
    assert_eq!(text(receiver.recv()).as_deref(), Some("line b"));
    assert_eq!(text(receiver.recv()).as_deref(), Some(r#"end Some("c")"#));
    assert_eq!(text(receiver.recv()), None);
}

#[test]
fn a_line_queued_after_a_dropped_one_is_the_last_line_again() {
    let (lines, end, receiver) = queue(1);
    lines.stop();
    lines.push("a".to_owned());
    lines.push("b".to_owned());

    assert_eq!(text(receiver.recv()).as_deref(), Some("line a"));
    lines.push("c".to_owned());
    end.send(ended());

    assert_eq!(text(receiver.recv()).as_deref(), Some("line c"));
    assert_eq!(text(receiver.recv()).as_deref(), Some("end None"));
}

#[test]
fn dropping_the_receiver_ends_a_push_that_waits_for_room() {
    let (lines, _end, receiver) = queue(1);
    lines.push("a".to_owned());

    let pushed = std::thread::scope(|scope| {
        let push = scope.spawn(|| lines.push("b".to_owned()));
        drop(receiver);
        push.join().unwrap()
    });

    assert!(!pushed);
}

#[test]
fn an_end_dropped_without_a_send_reaches_the_receiver_as_a_read_error() {
    let (lines, end, receiver) = queue(1);

    drop(end);

    assert!(lines.wait_ended(std::time::Instant::now()));
    let Some(Event::End { ended, .. }) = receiver.recv() else {
        panic!("the receiver got no end");
    };
    assert!(matches!(ended, Err(SpawnError::Read(_))));
}

#[test]
fn a_wait_with_a_deadline_is_silent_until_a_line_arrives_and_then_returns_it() {
    let (lines, _end, receiver) = queue(2);
    let past = std::time::Instant::now();

    let silent = receiver.recv_by(past);
    assert!(lines.push("a".to_owned()));
    let line = receiver.recv_by(past);

    assert!(silent.is_err());
    assert_eq!(text(line.unwrap()), Some("line a".to_owned()));
}
