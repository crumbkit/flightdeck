//! flightdeck's convention tests, which hold its command tree, tool record,
//! help pages, examples and failure codes to crumbkit's conventions with
//! crumbkit-test. They live in an integration test so that the checks that
//! run the built binary can find it. `SNAPSHOTS=overwrite cargo test`
//! rewrites the golden files under `tests/golden/` and the command
//! reference in `docs/commands.md`.

#![cfg(test)]

use clap::CommandFactory as _;
use crumbkit_flightdeck::{CODES, Cli, TOOL};
use crumbkit_test::{
    Allow, Examples, Failing, PathBudgets, Recoveries, Rule, Suite, Twins, Unprovoked,
};

/// The budgets of the paths through flightdeck's help, which are crumbkit's
/// defaults.
const PATH_BUDGETS: PathBudgets<'_> = PathBudgets::new("tests/golden/examples");

/// flightdeck's built binary and the directory that holds its example
/// golden files.
const EXAMPLES: Examples<'_> =
    Examples::new(env!("CARGO_BIN_EXE_flightdeck"), "tests/golden/examples");

/// The command line that provokes each code which a short run provokes.
const STARTS: &[Failing<'_>] = &[Failing::new(
    crumbkit_flightdeck::model::TOOL_NOT_FOUND,
    &["open", "./no-such-tool", "--no-browser"],
)];

/// The reason that no command of flightdeck provokes a code of the `run` or
/// `choices` area.
const NO_RUN: &str =
    "No flightdeck command runs a tool's commands yet; the runner returns the code.";

/// The reason that no command line provokes a workspaces code in the
/// convention tests: each needs an mxv that the test sets up.
const NEEDS_MXV: &str = "Only an mxv that fails, is missing or lacks the workspace provokes it, and a command line sets no variable; tests/workspaces.rs provokes it with a fake mxv.";

/// The reason that no command line provokes the workspace name code.
const NAME_FROM_PAGE: &str = "Only the page's route returns it; the command line refuses such a name as a usage error before flightdeck runs.";

/// The codes that the recovery check provokes with no command line.
const UNPROVOKED_RECOVERIES: &[Unprovoked<'_>] = &[
    Unprovoked::new(
        crumbkit_flightdeck::model::TOOL_LOAD,
        "Only a tool that prints something other than its description provokes it; tests/model.rs provokes it with fake tools.",
    ),
    Unprovoked::new(
        crumbkit_flightdeck::SERVER_FAILED,
        "Only a system that refuses a socket, a random read or a temporary file provokes it.",
    ),
    Unprovoked::new(
        crumbkit_flightdeck::TOOL_SEARCH_PATH,
        "Only a launch directory whose name holds PATH's separator provokes it; tests/open.rs provokes the code.",
    ),
    Unprovoked::new(
        crumbkit_flightdeck::ENVIRONMENT_INVALID,
        "Only a FLIGHTDECK_PORT that is not a port provokes it, and a command line sets no variable; tests/open.rs provokes the code.",
    ),
    Unprovoked::new(crumbkit_flightdeck::workspaces::MXV_MISSING, NEEDS_MXV),
    Unprovoked::new(crumbkit_flightdeck::workspaces::MXV_FAILED, NEEDS_MXV),
    Unprovoked::new(
        crumbkit_flightdeck::workspaces::WORKSPACE_NOT_FOUND,
        NEEDS_MXV,
    ),
    Unprovoked::new(
        crumbkit_flightdeck::workspaces::WORKSPACE_NAME,
        NAME_FROM_PAGE,
    ),
    Unprovoked::area("flightdeck", "run", NO_RUN),
    Unprovoked::area("flightdeck", "choices", NO_RUN),
];

/// The codes that have no twin: the admitted twin of a refused `open`
/// serves the page until it is stopped and never exits by itself, and no
/// command of flightdeck returns a run's code yet.
const UNTWINNED: &[Unprovoked<'_>] = &[
    Unprovoked::new(
        crumbkit_flightdeck::model::TOOL_NOT_FOUND,
        "Its admitted twin, flightdeck open shelf, serves until it is stopped; tests/open.rs provokes the code.",
    ),
    Unprovoked::new(
        crumbkit_flightdeck::model::TOOL_LOAD,
        "Only a tool that prints something other than its description provokes it; tests/model.rs provokes it with fake tools.",
    ),
    Unprovoked::new(
        crumbkit_flightdeck::SERVER_FAILED,
        "Only a system that refuses a socket, a random read or a temporary file provokes it.",
    ),
    Unprovoked::new(
        crumbkit_flightdeck::TOOL_SEARCH_PATH,
        "Only a launch directory whose name holds PATH's separator provokes it; tests/open.rs provokes the code.",
    ),
    Unprovoked::new(
        crumbkit_flightdeck::ENVIRONMENT_INVALID,
        "Only a FLIGHTDECK_PORT that is not a port provokes it, and a command line sets no variable; tests/open.rs provokes the code.",
    ),
    Unprovoked::new(crumbkit_flightdeck::workspaces::MXV_MISSING, NEEDS_MXV),
    Unprovoked::new(crumbkit_flightdeck::workspaces::MXV_FAILED, NEEDS_MXV),
    Unprovoked::new(
        crumbkit_flightdeck::workspaces::WORKSPACE_NOT_FOUND,
        NEEDS_MXV,
    ),
    Unprovoked::new(
        crumbkit_flightdeck::workspaces::WORKSPACE_NAME,
        NAME_FROM_PAGE,
    ),
    Unprovoked::area("flightdeck", "run", NO_RUN),
    Unprovoked::area("flightdeck", "choices", NO_RUN),
];

/// The rules that flightdeck skips for one command, each with its reason.
const ALLOW: &[Allow] = &[
    Allow::new(
        Rule::ListPages,
        "self environment list",
        "crumbkit-starmap's built-in prints every declared variable at once and takes no arguments.",
    ),
    Allow::new(
        Rule::ListPages,
        "workspaces list",
        "It prints the Workspaces page's document, which holds every workspace and counts them all.",
    ),
    Allow::new(
        Rule::ShowIds,
        "workspaces show",
        "It prints the One workspace page's document, which is about one workspace.",
    ),
];

/// The settings of flightdeck's recovery check.
const RECOVERIES: Recoveries<'_> = Recoveries::lists(EXAMPLES, CODES)
    .starts(STARTS)
    .unprovoked(UNPROVOKED_RECOVERIES);

/// The settings of flightdeck's twin check.
const TWINS: Twins<'_> = Twins::lists(EXAMPLES, CODES).unprovoked(UNTWINNED);

/// Everything the checks read, which `conventions!` expands to one test for
/// each check. The grammar check needs no verb besides crumbkit's own,
/// because `open` is a root verb of the tool record, and it accepts the
/// verbs of the `completion` commands.
const SUITE: Suite<'_> = Suite::new(|| (Cli::command(), TOOL))
    .allow(ALLOW)
    .verbs(&["install", "check", "print"])
    .path_budgets(PATH_BUDGETS)
    .examples(EXAMPLES)
    .recoveries(RECOVERIES)
    .twins(TWINS);

crumbkit_test::conventions!(SUITE);
