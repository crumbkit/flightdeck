use super::printable;

#[test]
fn printable_text_escapes_control_and_bidirectional_characters_and_keeps_the_rest() {
    let text = "shelf 1.0\u{1b}[2J\t\u{7f}\u{200f}\u{2066}é·";

    assert_eq!(
        printable(text),
        "shelf 1.0\\u{1b}[2J\\u{9}\\u{7f}\\u{200f}\\u{2066}é·"
    );
}
