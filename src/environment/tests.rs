use std::ffi::OsString;

use starmap::Env as _;

use super::Environment;

#[test]
fn the_struct_reads_exactly_the_variables_that_starmap_toml_declares() {
    starmap_test::check::<Environment>();
}

#[test]
fn an_empty_path_is_a_search_path_and_an_absent_one_is_none() {
    let empty = Environment::from_vars([(OsString::from("PATH"), OsString::new())]).unwrap();
    let absent = Environment::from_vars([]).unwrap();

    assert_eq!(empty.path(), Some(OsString::new().as_os_str()));
    assert_eq!(absent.path(), None);
}

#[test]
fn a_set_port_is_the_port_and_an_absent_one_is_none() {
    let set = Environment::from_vars([(OsString::from("FLIGHTDECK_PORT"), OsString::from("8123"))])
        .unwrap();
    let absent = Environment::from_vars([]).unwrap();

    assert_eq!(set.port(), Some(8123));
    assert_eq!(absent.port(), None);
}

#[test]
fn a_port_that_is_not_a_port_is_refused_and_the_refusal_names_the_variable() {
    for text in ["", "0", "65536", "http", "-1"] {
        let error =
            Environment::from_vars([(OsString::from("FLIGHTDECK_PORT"), OsString::from(text))])
                .unwrap_err();

        assert!(
            error.to_string().contains("FLIGHTDECK_PORT"),
            "{text}: {error}"
        );
    }
}

#[test]
fn the_lowest_and_highest_ports_are_ports() {
    for (text, port) in [("1", 1), ("65535", 65535)] {
        let environment =
            Environment::from_vars([(OsString::from("FLIGHTDECK_PORT"), OsString::from(text))])
                .unwrap();

        assert_eq!(environment.port(), Some(port));
    }
}

#[test]
fn a_set_mxv_is_its_program_and_an_absent_one_is_none() {
    let set = Environment::from_vars([(
        OsString::from("FLIGHTDECK_MXV"),
        OsString::from("/opt/mxv/bin/mxv"),
    )])
    .unwrap();
    let absent = Environment::from_vars([]).unwrap();

    assert_eq!(
        set.mxv(),
        Some(OsString::from("/opt/mxv/bin/mxv").as_os_str())
    );
    assert_eq!(absent.mxv(), None);
}
