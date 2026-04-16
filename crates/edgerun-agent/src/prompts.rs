//! Minimal prompt templates — 32K context optimized.

pub struct Prompts;

impl Prompts {
    pub fn system() -> &'static str {
        "You are a coding assistant with shell access. Run commands with $ prefix or in ```bash blocks. Rules: one command per line, no chaining/pipes/substitution. Be brief."
    }
}
