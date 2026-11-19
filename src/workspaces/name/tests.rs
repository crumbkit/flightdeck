use super::{NameError, WorkspaceName};

#[test]
fn a_plain_name_is_kept_as_given() {
    let name = WorkspaceName::new("reading-list").unwrap();

    assert_eq!(name.as_str(), "reading-list");
}

#[test]
fn an_empty_name_is_refused() {
    assert_eq!(WorkspaceName::new(""), Err(NameError::Empty));
}

#[test]
fn a_name_that_starts_with_a_dash_is_refused_so_it_never_reads_as_an_option() {
    assert_eq!(WorkspaceName::new("--all"), Err(NameError::Dash));
}

#[test]
fn a_name_with_a_control_character_is_refused() {
    assert_eq!(WorkspaceName::new("reading\nlist"), Err(NameError::Control));
    assert_eq!(
        WorkspaceName::new("reading\u{1b}list"),
        Err(NameError::Control)
    );
}

#[test]
fn a_name_with_a_slash_is_refused_because_a_workspace_is_one_directory() {
    assert_eq!(WorkspaceName::new("reading/list"), Err(NameError::Slash));
}

#[test]
fn the_name_parses_from_text_as_the_command_line_reads_it() {
    let name: WorkspaceName = "notes-scratch".parse().unwrap();

    assert_eq!(name.to_string(), "notes-scratch");
    assert_eq!(
        "-x".parse::<WorkspaceName>().unwrap_err().to_string(),
        "a workspace name cannot start with -"
    );
}
