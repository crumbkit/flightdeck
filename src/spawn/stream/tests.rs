use std::io::Cursor;

use super::{Delivered, Limit, Limits, deliver};

/// Delivers `output` with `limits` and returns the lines and how delivery
/// ended.
fn lines_of(output: &str, limits: Limits) -> (Vec<String>, Delivered) {
    let mut lines = Vec::new();
    let delivered = deliver(&mut Cursor::new(output), limits, &mut |line| {
        lines.push(line);
    })
    .unwrap();
    (lines, delivered)
}

#[test]
fn lines_are_split_at_each_newline_and_a_last_line_without_one_is_kept() {
    let (lines, delivered) = lines_of("{\"a\":1}\n{\"b\":2}\ntail", Limits::new(100, 100));

    assert_eq!(lines, ["{\"a\":1}", "{\"b\":2}", "tail"]);
    assert_eq!(delivered, Delivered::All);
}

#[test]
fn a_line_of_exactly_the_line_limit_arrives_and_one_byte_more_ends_delivery() {
    let limits = Limits::new(4, 100);

    let (within, delivered_within) = lines_of("abcd\n", limits);
    let (over, delivered_over) = lines_of("ab\nabcde\nab\n", limits);

    assert_eq!(within, ["abcd"]);
    assert_eq!(delivered_within, Delivered::All);
    assert_eq!(over, ["ab"]);
    assert_eq!(delivered_over, Delivered::Over(Limit::Line));
}

#[test]
fn output_past_the_total_limit_ends_delivery_after_the_lines_within_it() {
    let (lines, delivered) = lines_of("abc\nabc\nabc\n", Limits::new(100, 10));

    assert_eq!(lines, ["abc", "abc"]);
    assert_eq!(delivered, Delivered::Over(Limit::Total));
}
