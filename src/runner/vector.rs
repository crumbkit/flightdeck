//! Builds the argument vector of a run from a request, after checking each
//! part of the request against the tool's model.

use super::failure::{Reason, RunFailure, Takes};
use super::request::{RunRequest, Value};
use crate::model::{Argument, Arity, Command, CrumbkitFlag, Form, Kind, Tool, has_control};

#[cfg(test)]
mod tests;

/// The flags that flightdeck adds to a run, after the command's own
/// options, in this order: `--json` for a JSON document, on every run;
/// `--full` for every field of every row, when the command offers it; and
/// `--no-input` for no question, when the command takes it. crumbkit
/// refuses `--full` with `<tool>::cli::usage` on a command whose JSON
/// already holds every field, and a tool that cannot ask a question need
/// not declare `--no-input`.
const RUNNER_FLAGS: [CrumbkitFlag; 3] = [
    CrumbkitFlag::Json,
    CrumbkitFlag::Full,
    CrumbkitFlag::NoInput,
];

/// The checked vector of one request, from which a run's argument vector
/// and its display line are made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Vector {
    path: Vec<String>,
    options: Vec<String>,
    positionals: Vec<String>,
    line: Line,
    full: bool,
    no_input: bool,
    dry_run: bool,
    yes: bool,
}

/// What a run adds to a vector before its `--`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Extra {
    /// Nothing.
    None,
    /// `--dry-run`, for a preview.
    DryRun,
    /// `--yes`, for a confirmation, when the command takes it.
    Yes,
}

/// The argument vector of one run after the program, which only a
/// [`Vector`] makes, so the spawn module runs only checked commands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Argv {
    words: Vec<String>,
}

impl Argv {
    /// Returns the words of the vector.
    pub(crate) fn words(&self) -> &[String] {
        &self.words
    }
}

impl Vector {
    /// Returns the argument vector: the path, each option as one word, the
    /// flags of [`RUNNER_FLAGS`] that the command takes, `extra`, then `--`
    /// and the positional values.
    pub(super) fn argv(&self, extra: Extra) -> Argv {
        let mut words = self.path.clone();
        words.extend(self.options.iter().cloned());
        let added = RUNNER_FLAGS.into_iter().filter(|flag| match flag {
            CrumbkitFlag::Full => self.full,
            CrumbkitFlag::NoInput => self.no_input,
            _ => true,
        });
        words.extend(added.map(|flag| flag.name().to_owned()));
        match extra {
            Extra::DryRun => words.push(CrumbkitFlag::DryRun.name().to_owned()),
            Extra::Yes if self.yes => words.push(CrumbkitFlag::Yes.name().to_owned()),
            Extra::None | Extra::Yes => {}
        }
        words.push("--".to_owned());
        words.extend(self.positionals.iter().cloned());
        Argv { words }
    }

    /// Returns the command line as a person would type it, POSIX-quoted and
    /// without the flags that flightdeck adds, such as
    /// `shelf books remove 2`. With [`Extra::DryRun`] it holds `--dry-run`
    /// before the positional values, as the dry run ran. It never holds
    /// `--yes`, which stands for the answer a person gives in a terminal.
    pub(super) fn line(&self, extra: Extra) -> String {
        let flag = match extra {
            Extra::DryRun => format!(" {}", CrumbkitFlag::DryRun.name()),
            Extra::None | Extra::Yes => String::new(),
        };
        format!("{}{flag}{}", self.line.head, self.line.tail)
    }

    /// Returns true when the command takes `--dry-run`.
    pub(super) const fn takes_dry_run(&self) -> bool {
        self.dry_run
    }
}

/// Returns the command that `request` names and the vector of its values.
///
/// # Errors
///
/// Returns [`RunFailure::Refused`] when the tool lists no runnable command
/// at the path, when the command takes no argument of a name, when a name is
/// one of crumbkit's own flags other than `--limit` and `--all`, when a
/// value is of the wrong kind, is not one of the argument's possible values
/// or holds a control character, and when a positional value follows a
/// positional argument without one. The tool's model refused control
/// characters in command words and argument names when it loaded.
pub(super) fn build<'tool>(
    tool: &'tool Tool,
    request: &RunRequest,
) -> Result<(&'tool Command, Vector), RunFailure> {
    let path = request.path();
    let words: Vec<&str> = path.iter().map(String::as_str).collect();
    let command = tool
        .command(&words)
        .ok_or_else(|| RunFailure::refused(path, None, Reason::UnknownCommand))?;
    let refuse = |argument: &str, reason| RunFailure::refused(path, Some(argument), reason);

    for name in request.values().keys() {
        let argument = command
            .arguments()
            .iter()
            .find(|argument| argument.name() == name)
            .ok_or_else(|| refuse(name, Reason::UnknownArgument))?;
        if argument
            .crumbkit_flag()
            .is_some_and(|flag| !matches!(flag, CrumbkitFlag::Limit | CrumbkitFlag::All))
        {
            return Err(refuse(name, Reason::Reserved));
        }
    }

    let mut line = Line::new(tool.name(), path);
    let mut options = Vec::new();
    let mut positionals = Vec::new();
    // The first positional argument without a value. A value of a later
    // positional argument would reach the tool as this one's.
    let mut empty: Option<&str> = None;
    for argument in command.arguments() {
        let given = match request.values().get(argument.name()) {
            Some(value) => {
                given(argument, value).map_err(|reason| refuse(argument.name(), reason))?
            }
            None => Given {
                flag: false,
                values: Vec::new(),
            },
        };
        if argument.kind() == Kind::Positional && !given.flag {
            match (empty, given.values.is_empty()) {
                (None, true) => empty = Some(argument.name()),
                (Some(earlier), false) => {
                    let earlier = earlier.to_owned();
                    return Err(refuse(argument.name(), Reason::Skipped { earlier }));
                }
                (None | Some(_), _) => {}
            }
        }
        if given.values.is_empty() && !given.flag {
            continue;
        }
        // A list value names its item at fault, so the page can mark it.
        let listed = matches!(argument.arity(), Arity::Many { .. });
        for (index, value) in given.values.iter().enumerate() {
            let index = listed.then_some(index);
            if has_control(value) {
                return Err(refuse(argument.name(), Reason::Control { index }));
            }
            let possible = argument.possible_values();
            if !possible.is_empty() && !possible.contains(value) {
                return Err(refuse(argument.name(), Reason::NotPossible { index }));
            }
        }
        match (argument.kind(), given.flag) {
            (Kind::Named, true) => {
                options.push(argument.name().to_owned());
                line.flag(argument.name());
            }
            (Kind::Named, false) => {
                for value in given.values {
                    options.push(format!("{}={value}", argument.name()));
                    line.option(argument.name(), &value);
                }
            }
            // crumbkit gives every positional argument a value name, so a
            // positional flag is no argument that a command line can set.
            (Kind::Positional, true) => {
                return Err(refuse(argument.name(), Reason::UnknownArgument));
            }
            (Kind::Positional, false) => positionals.extend(given.values),
        }
    }
    line.positionals(&positionals);

    let takes = |flag: CrumbkitFlag| command.crumbkit_flags().any(|(own, _)| own == flag);
    // crumbkit lists a global `--full` on every command below the one that
    // declares it, also on a command that refuses it. A headed or bytes
    // result never offers `--full`, so the form rules those out.
    let whole = matches!(command.result().form(), Some(Form::Headed | Form::Bytes));
    let vector = Vector {
        path: request.path().to_vec(),
        options,
        positionals,
        line,
        full: takes(CrumbkitFlag::Full) && !whole,
        no_input: takes(CrumbkitFlag::NoInput),
        dry_run: takes(CrumbkitFlag::DryRun),
        yes: takes(CrumbkitFlag::Yes),
    };
    Ok((command, vector))
}

/// What the page gave for one argument: a flag that is set, or values.
struct Given {
    flag: bool,
    values: Vec<String>,
}

/// Returns what `value` gives for `argument`, or why it is of the wrong
/// kind. A false flag and an empty list give nothing.
fn given(argument: &Argument, value: &Value) -> Result<Given, Reason> {
    let none = || Given {
        flag: false,
        values: Vec::new(),
    };
    let takes = match argument.arity() {
        Arity::Flag | Arity::Count => Takes::Flag,
        Arity::One { .. } => Takes::One,
        Arity::Many { .. } => Takes::Many,
    };
    if argument.arity().value_names().len() > 1 {
        return Err(Reason::Grouped);
    }
    match (takes, value) {
        // RUST-10: a counted flag, such as `-vv`, is set once. It handles the
        // page's checkbox, and it misses a count above one. A number value
        // replaces this when a tool has a counted flag that the page shows.
        (Takes::Flag, Value::Flag(true)) => Ok(Given {
            flag: true,
            values: Vec::new(),
        }),
        (Takes::Flag, Value::Flag(false)) => Ok(none()),
        (Takes::One, Value::One(one)) => Ok(Given {
            flag: false,
            values: vec![one.clone()],
        }),
        (Takes::Many, Value::Many(many)) => Ok(Given {
            flag: false,
            values: many.clone(),
        }),
        _ => Err(Reason::Arity { takes }),
    }
}

/// The display line of a vector: the head, with the tool's name, the path
/// and the options, and the tail, with the positional values. A flag of a
/// run goes between the two.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Line {
    head: String,
    tail: String,
}

impl Line {
    /// Starts the line with the tool's name and the command's path.
    fn new(tool: &str, path: &[String]) -> Self {
        let mut head = quote_or_empty(tool);
        for word in path {
            push(&mut head, word);
        }
        Self {
            head,
            tail: String::new(),
        }
    }

    /// Adds a flag, such as `--all`.
    fn flag(&mut self, name: &str) {
        push(&mut self.head, name);
    }

    /// Adds an option as one word, such as `--limit=10` or
    /// `--from='my ids.txt'`.
    fn option(&mut self, name: &str, value: &str) {
        let word = format!("{}={}", quote_or_empty(name), quote_or_empty(value));
        self.head.push(' ');
        self.head.push_str(&word);
    }

    /// Adds the positional values, after `--` when one of them starts with
    /// `-`.
    fn positionals(&mut self, values: &[String]) {
        if values.iter().any(|value| value.starts_with('-')) {
            push(&mut self.tail, "--");
        }
        for value in values {
            push(&mut self.tail, value);
        }
    }
}

/// Adds one quoted word to `text`.
fn push(text: &mut String, word: &str) {
    text.push(' ');
    text.push_str(&quote_or_empty(word));
}

/// Returns `word` quoted for a POSIX shell. A word that holds a NUL byte,
/// which no shell word can hold, prints as the empty word `''`. The model
/// refuses a tool whose name, command words or argument names hold a
/// control character, NUL among them, when it loads, and `build` refuses
/// every such value, so that word never prints.
fn quote_or_empty(word: &str) -> String {
    shlex::try_quote(word).map_or_else(|_| "''".to_owned(), |quoted| quoted.into_owned())
}
