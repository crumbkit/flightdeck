//! One argument of a command, and crumbkit's own flags among them.

use serde::Serialize;

use super::wire::ArgumentRow;

/// One argument of a command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Argument {
    name: String,
    short: Option<String>,
    kind: Kind,
    arity: Arity,
    required: bool,
    defaults: Vec<String>,
    possible_values: Vec<String>,
    help: Help,
    one_of: Vec<String>,
    global: bool,
    crumbkit_flag: Option<CrumbkitFlag>,
}

impl Argument {
    /// Returns the argument of a `commands list` row.
    pub(super) fn from_row(row: ArgumentRow) -> Self {
        let kind = if row.name.starts_with('-') {
            Kind::Named
        } else {
            Kind::Positional
        };
        let arity = match (row.value_names.is_empty(), row.many) {
            (true, false) => Arity::Flag,
            (true, true) => Arity::Count,
            (false, false) => Arity::One {
                value_names: row.value_names,
            },
            (false, true) => Arity::Many {
                value_names: row.value_names,
            },
        };
        Self {
            crumbkit_flag: CrumbkitFlag::from_name(&row.name),
            name: row.name,
            short: row.short,
            kind,
            arity,
            required: row.required,
            defaults: row.defaults,
            possible_values: row.possible_values,
            help: Help {
                line: row.help,
                long: row.long_help,
            },
            one_of: row.one_of,
            global: row.global,
        }
    }

    /// Returns the argument's name as help writes it: `--limit` for an
    /// option, `<ID>` for a positional argument.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the argument's short form, such as `-n`.
    #[must_use]
    pub fn short(&self) -> Option<&str> {
        self.short.as_deref()
    }

    /// Returns whether the argument is named or positional.
    #[must_use]
    pub const fn kind(&self) -> Kind {
        self.kind
    }

    /// Returns how many values the argument takes.
    #[must_use]
    pub const fn arity(&self) -> &Arity {
        &self.arity
    }

    /// Returns true when the command needs the argument, unless one of the
    /// arguments in [`Argument::one_of`] stands in for it.
    #[must_use]
    pub const fn required(&self) -> bool {
        self.required
    }

    /// Returns the values the argument takes when it is left out.
    #[must_use]
    pub fn defaults(&self) -> &[String] {
        &self.defaults
    }

    /// Returns the only values the argument accepts, or none when it
    /// accepts any value.
    #[must_use]
    pub fn possible_values(&self) -> &[String] {
        &self.possible_values
    }

    /// Returns the argument's help.
    #[must_use]
    pub const fn help(&self) -> &Help {
        &self.help
    }

    /// Returns the arguments that can stand in for this one, such as
    /// `--from` for the ids of `books show`.
    #[must_use]
    pub fn one_of(&self) -> &[String] {
        &self.one_of
    }

    /// Returns true when the argument is global, so every command below the
    /// group that defines it takes it.
    #[must_use]
    pub const fn global(&self) -> bool {
        self.global
    }

    /// Returns which of crumbkit's own flags the argument is, or `None` for
    /// the tool's own argument.
    #[must_use]
    pub const fn crumbkit_flag(&self) -> Option<CrumbkitFlag> {
        self.crumbkit_flag
    }
}

/// Whether an argument is named or positional.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// An option or a flag, such as `--limit`.
    Named,
    /// A positional argument, such as `<ID>`.
    Positional,
}

/// How many values an argument takes. It serializes with its variant's
/// name in `kind`, such as `{"kind": "many", "value_names": ["ID"]}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Arity {
    /// A flag that takes no value.
    Flag,
    /// A flag that takes no value and counts its repeats, such as `-vv`.
    Count,
    /// One value, or one group of values with these names.
    One {
        /// The names of the values, such as `["N"]`.
        value_names: Vec<String>,
    },
    /// Any number of values with these names.
    Many {
        /// The names of the values, such as `["ID"]`.
        value_names: Vec<String>,
    },
}

impl Arity {
    /// Returns the names of the values the argument takes, or none for a
    /// flag.
    #[must_use]
    pub fn value_names(&self) -> &[String] {
        match self {
            Self::Flag | Self::Count => &[],
            Self::One { value_names } | Self::Many { value_names } => value_names,
        }
    }
}

/// The help of an argument.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Help {
    line: Option<String>,
    long: Option<String>,
}

impl Help {
    /// Returns the one line that `-h` prints.
    #[must_use]
    pub fn line(&self) -> Option<&str> {
        self.line.as_deref()
    }

    /// Returns the longer text that `--help` prints, when the argument has
    /// one.
    #[must_use]
    pub fn long(&self) -> Option<&str> {
        self.long.as_deref()
    }
}

/// One of crumbkit's own flags, which flightdeck sets itself and never
/// shows as a form field. It serializes as its name, such as `--json`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(into = "&'static str")]
pub enum CrumbkitFlag {
    /// `--json`, which prints the result as a JSON document.
    Json,
    /// `--fields`, which chooses the columns.
    Fields,
    /// `--long`, which adds the long columns.
    Long,
    /// `--full`, which prints every field of every row with `--json`.
    Full,
    /// `--no-input`, which fails instead of asking a question.
    NoInput,
    /// `--dry-run`, which prints what a command would change.
    DryRun,
    /// `--yes`, which confirms a change without asking.
    Yes,
    /// `--limit`, which sets the page size.
    Limit,
    /// `--all`, which prints every row.
    All,
}

impl CrumbkitFlag {
    /// Every one of crumbkit's own flags.
    pub const ALL: [Self; 9] = [
        Self::Json,
        Self::Fields,
        Self::Long,
        Self::Full,
        Self::NoInput,
        Self::DryRun,
        Self::Yes,
        Self::Limit,
        Self::All,
    ];

    /// Returns the flag's name, such as `--json`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Json => "--json",
            Self::Fields => "--fields",
            Self::Long => "--long",
            Self::Full => "--full",
            Self::NoInput => "--no-input",
            Self::DryRun => "--dry-run",
            Self::Yes => "--yes",
            Self::Limit => "--limit",
            Self::All => "--all",
        }
    }

    /// Returns the flag named `name`, or `None` when crumbkit has no flag of
    /// that name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|flag| flag.name() == name)
    }
}

impl From<CrumbkitFlag> for &'static str {
    fn from(flag: CrumbkitFlag) -> Self {
        flag.name()
    }
}
