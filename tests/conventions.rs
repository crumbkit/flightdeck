//! flightdeck's convention tests, which hold its command tree, guide, help
//! pages, examples and failure codes to crumbkit's conventions with
//! crumbkit-test. They live in an integration test so that the checks that
//! run the built binary can find it. `SNAPSHOTS=overwrite cargo test`
//! rewrites the golden files under `tests/golden/` and the command
//! reference in `docs/commands.md`.

#![cfg(test)]

use clap::CommandFactory as _;
use crumbkit_flightdeck::{CODES, Cli, GUIDE};
use crumbkit_test::{Allow, Budgets, Examples, Pages, PathBudgets, Recoveries, Twins};

/// flightdeck's opt-outs from crumbkit's conventions. It has none.
const ALLOW: &[Allow] = &[];

/// The budgets of flightdeck's `-h` pages, which are crumbkit's defaults.
const BUDGETS: Budgets = Budgets::new();

/// The budgets of the paths through flightdeck's help, which are crumbkit's
/// defaults.
const PATH_BUDGETS: PathBudgets<'_> = PathBudgets::new("tests/golden/examples");

/// flightdeck's built binary and the directory that holds its example
/// golden files.
const EXAMPLES: Examples<'_> =
    Examples::new(env!("CARGO_BIN_EXE_flightdeck"), "tests/golden/examples").allow(ALLOW);

#[test]
fn every_command_and_argument_has_one_name() {
    crumbkit_test::check_names(&Cli::command(), Pages::rendered(&GUIDE), ALLOW);
}

#[test]
fn every_command_is_a_resource_followed_by_a_verb() {
    crumbkit_test::check_grammar(&Cli::command(), Pages::rendered(&GUIDE), &[], ALLOW);
}

#[test]
fn every_command_declares_its_effects() {
    crumbkit_test::check_effects(&Cli::command(), &GUIDE, ALLOW);
}

#[test]
fn every_list_command_pages_and_every_show_command_takes_up_to_100_ids() {
    crumbkit_test::check_listings(&Cli::command(), ALLOW);
}

#[test]
fn every_option_with_many_values_accepts_repeats_and_commas() {
    crumbkit_test::check_filters(&Cli::command(), ALLOW);
}

#[test]
fn every_command_and_leaf_meets_the_guide_rules() {
    crumbkit_test::check_guide(&Cli::command(), &GUIDE, ALLOW);
}

#[test]
fn every_exit_code_meets_the_exit_code_rules() {
    crumbkit_test::check_exit_codes(&Cli::command(), &GUIDE, ALLOW);
}

#[test]
fn every_failure_code_is_distinct_and_printable() {
    crumbkit_test::check_codes(&GUIDE, &[CODES]);
}

#[test]
fn every_help_page_fits_its_size_budget() {
    crumbkit_test::check_help_sizes(&Cli::command(), Pages::rendered(&GUIDE), &BUDGETS);
}

#[test]
fn every_path_through_the_help_fits_its_weight_budget() {
    crumbkit_test::check_path_weights(&Cli::command(), Pages::rendered(&GUIDE), &PATH_BUDGETS);
}

#[test]
fn every_help_page_matches_its_golden_file() {
    crumbkit_test::check_help_goldens(
        &Cli::command(),
        Pages::rendered(&GUIDE),
        "tests/golden/help",
    );
}

#[test]
fn every_document_schema_matches_its_golden_file() {
    crumbkit_test::check_schema_goldens(&Cli::command(), &GUIDE, "tests/golden/schemas");
}

#[test]
fn every_document_is_named_after_the_path_of_the_command_that_prints_it() {
    crumbkit_test::check_document_ids(&Cli::command(), &GUIDE, ALLOW);
}

#[test]
fn every_crumbkit_command_sits_at_its_fixed_path() {
    crumbkit_test::check_mounts(&Cli::command(), &GUIDE);
}

#[test]
fn every_example_in_the_guide_runs_and_prints_its_golden_output() {
    crumbkit_test::check_examples(&Cli::command(), &GUIDE, &EXAMPLES);
}

#[test]
fn every_json_example_cuts_or_refuses_a_result_past_its_output_budget() {
    crumbkit_test::check_output_budget(&Cli::command(), &GUIDE, &EXAMPLES);
}

#[test]
fn every_command_matches_the_checked_in_command_reference() {
    crumbkit_test::check_command_reference(env!("CARGO_BIN_EXE_flightdeck"), "docs/commands.md");
}

#[test]
fn every_opt_out_matches_the_opt_out_report() {
    crumbkit_test::check_opt_outs(
        &Cli::command(),
        &GUIDE,
        ALLOW,
        &BUDGETS,
        Some(&PATH_BUDGETS),
        "tests/golden",
    );
}

#[test]
fn every_recovery_leads_to_a_command_that_runs() {
    let recoveries = Recoveries::new(EXAMPLES, &[CODES]);
    crumbkit_test::check_recoveries(&Cli::command(), &GUIDE, &recoveries, "tests/golden");
}

#[test]
fn every_declared_code_has_a_twin_that_the_tool_accepts() {
    let twins = Twins::new(EXAMPLES, &[CODES]);
    crumbkit_test::check_twins(&Cli::command(), &GUIDE, &twins);
}
