#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

use alloc::string::String;
use alloc::vec::Vec;
use edgerun_json::Value;

pub struct Command {
    name: String,
    about: Option<String>,
    args: Vec<Arg>,
    subcommands: Vec<Command>,
}

impl Command {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            about: None,
            args: Vec::new(),
            subcommands: Vec::new(),
        }
    }

    pub fn about(mut self, about: impl Into<String>) -> Self {
        self.about = Some(about.into());
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

    pub fn get_matches(&self) -> ArgMatches {
        let mut args = std::env::args().skip(1).collect::<Vec<_>>();
        let mut matches = ArgMatches::new();

        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];
            if arg.starts_with('-') {
                let key = arg.trim_start_matches('-');
                if let Some(a) = self.args.iter().find(|a| a.long.as_deref() == Some(key) || a.short.map(|s| s.to_string() == key).unwrap_or(false)) {
                    i += 1;
                    if i < args.len() && !args[i].starts_with('-') {
                        matches.map.insert(a.name.clone(), Value::String(args[i].clone()));
                        i += 1;
                    } else {
                        matches.map.insert(a.name.clone(), Value::Bool(true));
                    }
                }
            } else {
                matches.positional.push(arg.clone());
                i += 1;
            }
        }

        matches
    }
}

pub struct Arg {
    name: String,
    short: Option<char>,
    long: Option<String>,
    help: Option<String>,
    default_value: Option<String>,
}

impl Arg {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            short: None,
            long: None,
            help: None,
            default_value: None,
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
}

pub struct ArgMatches {
    pub(crate) map: alloc::collections::BTreeMap<String, Value>,
    pub positional: alloc::vec::Vec<String>,
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
        T: core::str::FromStr,
        <T as core::str::FromStr>::Err: core::fmt::Debug,
    {
        self.map.get(name).and_then(|v| match v {
            Value::String(s) => s.parse().ok(),
            Value::Number(n) => n.to_string().parse().ok(),
            Value::Bool(b) => if *b { Some(T::from_str("true").ok()?) } else { None },
            _ => None,
        })
    }

    pub fn contains_id(&self, id: &str) -> bool {
        self.map.contains_key(id)
    }
}

pub struct Parser;

#[cfg(feature = "std")]
impl Parser {
    pub fn parse<T: FromArgMatches>() -> T {
        let cmd = T::command();
        let matches = cmd.get_matches();
        T::from(&matches)
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