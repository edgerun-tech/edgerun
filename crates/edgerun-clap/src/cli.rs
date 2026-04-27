#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use edgerun_json::{JsonNumber, Value};

pub struct Command {
    name: String,
    about: Option<String>,
    version: Option<String>,
    author: Option<String>,
    args: Vec<Arg>,
    subcommands: Vec<Command>,
    action: Option<Action>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    Set,
    StoreTrue,
    StoreFalse,
    Append,
    Count,
}

impl Default for Action {
    fn default() -> Self {
        Self::Set
    }
}

impl Command {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            about: None,
            version: None,
            author: None,
            args: Vec::new(),
            subcommands: Vec::new(),
            action: None,
        }
    }

    pub fn about(mut self, about: impl Into<String>) -> Self {
        self.about = Some(about.into());
        self
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    pub fn arg(mut self, arg: Arg) -> Self {
        self.args.push(arg);
        self
    }

    pub fn subcommand(mut self, subcommand: Command) -> Self {
        self.subcommands.push(subcommand);
        self
    }

    pub fn action(mut self, action: Action) -> Self {
        self.action = Some(action);
        self
    }

    #[cfg(feature = "std")]
    pub fn get_matches(&self) -> ArgMatches {
        self.get_matches_from(std::env::args().skip(1).collect())
    }

    pub fn get_matches_from(&self, args: Vec<String>) -> ArgMatches {
        let mut matches = ArgMatches::new();
        let mut positional_values: Vec<String> = Vec::new();
        let mut i = 0;

        while i < args.len() {
            let arg = &args[i];

            if arg == "--" {
                for rem in args.iter().skip(i + 1) {
                    positional_values.push(rem.clone());
                }
                break;
            }

            if arg.starts_with('-') && arg.len() > 1 {
                let rest = if arg.starts_with("--") {
                    arg.trim_start_matches('-')
                } else {
                    arg.trim_start_matches('-')
                };

                if rest.is_empty() {
                    i += 1;
                    continue;
                }

                if let Some(a) = self.args.iter().find(|a| {
                    a.long.as_deref() == Some(rest)
                        || a.short.map(|s| s.to_string() == rest).unwrap_or(false)
                }) {
                    match a.action {
                        Some(Action::StoreTrue) => {
                            matches.map.insert(a.name.clone(), Value::Bool(true));
                        }
                        Some(Action::StoreFalse) => {
                            matches.map.insert(a.name.clone(), Value::Bool(false));
                        }
                        Some(Action::Count) => {
                            let current = matches
                                .map
                                .get(&a.name)
                                .and_then(|v| match v {
                                    Value::Number(n) => n.as_i64(),
                                    _ => None,
                                })
                                .unwrap_or(0);
                            matches.map.insert(
                                a.name.clone(),
                                Value::Number(JsonNumber::I64(current + 1)),
                            );
                        }
                        _ => {
                            i += 1;
                            if let Some(delimiter) = a.value_delimiter {
                                let values: Vec<_> = if i < args.len() && !args[i].starts_with('-')
                                {
                                    args[i].split(delimiter).map(String::from).collect()
                                } else {
                                    Vec::new()
                                };
                                if values.len() == 1 {
                                    matches
                                        .map
                                        .insert(a.name.clone(), Value::String(values[0].clone()));
                                } else {
                                    matches.map.insert(
                                        a.name.clone(),
                                        Value::Array(
                                            values.into_iter().map(Value::String).collect(),
                                        ),
                                    );
                                }
                            } else if i < args.len() && !args[i].starts_with('-') {
                                matches
                                    .map
                                    .insert(a.name.clone(), Value::String(args[i].clone()));
                            } else if a.num_args.is_none() {
                                matches.map.insert(a.name.clone(), Value::Bool(true));
                            } else {
                                let mut values = Vec::new();
                                for _ in 0..a.num_args.unwrap_or(1) {
                                    if i < args.len() && !args[i].starts_with('-') {
                                        values.push(args[i].clone());
                                        i += 1;
                                    }
                                }
                                if values.is_empty() {
                                    matches.map.insert(a.name.clone(), Value::Bool(true));
                                } else if values.len() == 1 {
                                    matches
                                        .map
                                        .insert(a.name.clone(), Value::String(values[0].clone()));
                                } else {
                                    matches.map.insert(
                                        a.name.clone(),
                                        Value::Array(
                                            values.into_iter().map(Value::String).collect(),
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                positional_values.push(arg.clone());
            }
            i += 1;
        }

        matches.positional = positional_values;
        matches
    }

    #[cfg(feature = "std")]
    pub fn print_help(&self) {
        eprintln!("Usage: {} [OPTIONS] [SUBCOMMAND]", self.name);

        if let Some(about) = &self.about {
            eprintln!();
            eprintln!("{}", about);
        }

        if !self.args.iter().any(|a| a.is_positional) {
            if !self.args.is_empty() {
                eprintln!();
                eprintln!("Options:");
                for arg in &self.args {
                    if let Some(long) = &arg.long {
                        let mut flags = String::new();
                        if let Some(short) = arg.short {
                            flags.push('-');
                            flags.push(short);
                            flags.push_str(", ");
                        }
                        flags.push_str("--");
                        flags.push_str(long);

                        let is_store_bool = matches!(
                            arg.action,
                            Some(Action::StoreTrue) | Some(Action::StoreFalse)
                        );
                        if let Some(default) = &arg.default_value {
                            eprintln!("  {} (default: {})", flags, default);
                        } else if is_store_bool {
                            eprintln!("  {}", flags);
                        } else if arg.required {
                            eprintln!("  {} <value> (required)", flags);
                        } else {
                            eprintln!("  {}", flags);
                        }

                        if let Some(help) = &arg.help {
                            eprintln!("    {}", help);
                        }
                    }
                }
            }
        } else {
            eprintln!();
            eprintln!("Arguments:");
            for arg in &self.args {
                if arg.is_positional {
                    if arg.required {
                        eprintln!("  {}", arg.name.to_uppercase());
                    } else {
                        eprintln!("  [{}]", arg.name.to_uppercase());
                    }
                    if let Some(help) = &arg.help {
                        eprintln!("    {}", help);
                    }
                }
            }
        }

        if !self.subcommands.is_empty() {
            eprintln!();
            eprintln!("Subcommands:");
            for sub in &self.subcommands {
                eprintln!("  {}", sub.name);
                if let Some(about) = &sub.about {
                    eprintln!("    {}", about);
                }
            }
        }
    }

    pub fn find_subcommand(&self, name: &str) -> Option<&Command> {
        self.subcommands.iter().find(|s| s.name == name)
    }
}

pub struct Arg {
    name: String,
    short: Option<char>,
    long: Option<String>,
    help: Option<String>,
    default_value: Option<String>,
    num_args: Option<usize>,
    value_delimiter: Option<char>,
    is_subcommand: bool,
    is_positional: bool,
    required: bool,
    action: Option<Action>,
}

impl Arg {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            short: None,
            long: None,
            help: None,
            default_value: None,
            num_args: None,
            value_delimiter: None,
            is_subcommand: false,
            is_positional: false,
            required: false,
            action: None,
        }
    }

    pub fn short(mut self, short: char) -> Self {
        self.short = Some(short);
        self
    }

    pub fn long(mut self, long: impl Into<String>) -> Self {
        self.long = Some(long.into());
        self
    }

    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn default_value(mut self, default: impl Into<String>) -> Self {
        self.default_value = Some(default.into());
        self
    }

    pub fn num_args(mut self, n: usize) -> Self {
        self.num_args = Some(n);
        self
    }

    pub fn value_delimiter(mut self, d: char) -> Self {
        self.value_delimiter = Some(d);
        self
    }

    pub fn subcommand(mut self) -> Self {
        self.is_subcommand = true;
        self
    }

    pub fn positional(mut self) -> Self {
        self.is_positional = true;
        self.long = Some(self.name.clone());
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn action(mut self, action: Action) -> Self {
        self.action = Some(action);
        self
    }
}

pub struct ArgMatches {
    pub(crate) map: alloc::collections::BTreeMap<String, Value>,
    pub positional: alloc::vec::Vec<String>,
}

impl Default for ArgMatches {
    fn default() -> Self {
        Self::new()
    }
}

impl ArgMatches {
    pub fn new() -> Self {
        Self {
            map: alloc::collections::BTreeMap::new(),
            positional: alloc::vec::Vec::new(),
        }
    }

    pub fn get_one<T: core::str::FromStr>(&self, name: &str) -> Option<T>
    where
        <T as core::str::FromStr>::Err: core::fmt::Debug,
    {
        self.map.get(name).and_then(|v| match v {
            Value::String(s) => s.parse().ok(),
            Value::Number(n) => n.to_string().parse().ok(),
            Value::Bool(b) => {
                if *b {
                    Some(T::from_str("true").ok()?)
                } else {
                    None
                }
            }
            _ => None,
        })
    }

    pub fn get_many<T: core::str::FromStr>(&self, name: &str) -> Option<alloc::vec::Vec<T>>
    where
        <T as core::str::FromStr>::Err: core::fmt::Debug,
    {
        self.map.get(name).and_then(|v| match v {
            Value::Array(arr) => {
                let mut result = alloc::vec::Vec::new();
                for item in arr {
                    if let Value::String(s) = item {
                        if let Ok(parsed) = s.parse::<T>() {
                            result.push(parsed);
                        }
                    }
                }
                if result.is_empty() {
                    None
                } else {
                    Some(result)
                }
            }
            Value::String(s) => s.parse::<T>().ok().map(|v| alloc::vec![v]),
            _ => None,
        })
    }

    pub fn get_positional(&self, index: usize) -> Option<&str> {
        self.positional.get(index).map(|s| s.as_str())
    }

    pub fn positional_count(&self) -> usize {
        self.positional.len()
    }

    pub fn contains_id(&self, id: &str) -> bool {
        self.map.contains_key(id)
    }

    pub fn get_flag(&self, name: &str) -> bool {
        self.map
            .get(name)
            .and_then(|v| match v {
                Value::Bool(b) => Some(*b),
                Value::Number(n) => n.as_i64().map(|n| n != 0),
                _ => None,
            })
            .unwrap_or(false)
    }
}

pub trait FromArgMatches {
    fn command() -> Command;
    fn from(matches: &ArgMatches) -> Self;
}

pub struct ArgGroup {
    name: String,
}

impl ArgGroup {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

pub trait Parser: Sized {
    fn command() -> Command;
    #[cfg(feature = "std")]
    fn parse() -> Self {
        let matches = Self::command().get_matches();
        Self::from(&matches)
    }
    fn from(matches: &ArgMatches) -> Self;
}

pub trait Subcommand: Sized {
    fn name() -> &'static str;
}
