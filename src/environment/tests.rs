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
