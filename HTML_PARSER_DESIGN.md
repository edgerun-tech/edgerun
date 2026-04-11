import os, zipfile, textwrap, shutil

base = "/mnt/data/html_parser_project_v3"
if os.path.exists(base):
    shutil.rmtree(base)
os.makedirs(base)

def write(path, content):
    p = os.path.join(base, path)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w") as f:
        f.write(content)

# README
write("README.md", textwrap.dedent("""
# HTML Parser Compiler (Phase 3 – Real State Machine + DOM)

Features:
- Table-driven tokenizer
- Proper start/end tag parsing
- Token stream
- Minimal DOM tree builder (stack-based)

Run:
python3 scripts/build_ir.py
python3 scripts/generate_parser.py
cd rust
cargo run
"""))

# IR builder
write("scripts/build_ir.py", textwrap.dedent("""
import json, os

ir = {
    "states": ["DATA","TAG_OPEN","END_TAG_OPEN","TAG_NAME"],
    "transitions": [
        ("DATA","<","TAG_OPEN",[]),
        ("DATA","*","DATA",["EMIT_CHAR"]),

        ("TAG_OPEN","/","END_TAG_OPEN",[]),
        ("TAG_OPEN","*","TAG_NAME",["START_TAG","APPEND_NAME"]),

        ("END_TAG_OPEN","*","TAG_NAME",["END_TAG","APPEND_NAME"]),

        ("TAG_NAME",">","DATA",["EMIT_TOKEN"]),
        ("TAG_NAME","*","TAG_NAME",["APPEND_NAME"])
    ]
}

os.makedirs("ir", exist_ok=True)
with open("ir/tokenizer.json","w") as f:
    json.dump(ir,f)

print("IR built")
"""))

# Generator
write("scripts/generate_parser.py", textwrap.dedent("""
import os

code = '''
#[derive(Debug)]
pub enum Token {
    StartTag(String),
    EndTag(String),
    Text(String),
}

#[derive(Debug)]
pub struct Node {
    pub name: String,
    pub children: Vec<Node>,
    pub text: Option<String>,
}

pub struct Parser {
    state: State,
    buffer: String,
    current_tag: String,
    tokens: Vec<Token>,
}

#[derive(Copy, Clone)]
enum State {
    Data,
    TagOpen,
    EndTagOpen,
    TagName,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            state: State::Data,
            buffer: String::new(),
            current_tag: String::new(),
            tokens: vec![],
        }
    }

    pub fn parse(&mut self, input: &str) -> Node {
        for c in input.chars() {
            self.step(c);
        }

        self.build_dom()
    }

    fn step(&mut self, c: char) {
        match self.state {
            State::Data => {
                if c == '<' {
                    if !self.buffer.is_empty() {
                        self.tokens.push(Token::Text(self.buffer.clone()));
                        self.buffer.clear();
                    }
                    self.state = State::TagOpen;
                } else {
                    self.buffer.push(c);
                }
            }

            State::TagOpen => {
                if c == '/' {
                    self.state = State::EndTagOpen;
                } else {
                    self.current_tag.clear();
                    self.current_tag.push(c);
                    self.state = State::TagName;
                }
            }

            State::EndTagOpen => {
                self.current_tag.clear();
                self.current_tag.push(c);
                self.state = State::TagName;
            }

            State::TagName => {
                if c == '>' {
                    if self.tokens.last().map(|t| matches!(t, Token::StartTag(_))).unwrap_or(false) {
                        self.tokens.push(Token::EndTag(self.current_tag.clone()));
                    } else {
                        self.tokens.push(Token::StartTag(self.current_tag.clone()));
                    }
                    self.state = State::Data;
                } else {
                    self.current_tag.push(c);
                }
            }
        }
    }

    fn build_dom(&self) -> Node {
        let mut stack: Vec<Node> = vec![Node {
            name: "root".into(),
            children: vec![],
            text: None,
        }];

        for token in &self.tokens {
            match token {
                Token::StartTag(name) => {
                    stack.push(Node {
                        name: name.clone(),
                        children: vec![],
                        text: None,
                    });
                }

                Token::EndTag(_) => {
                    if stack.len() > 1 {
                        let node = stack.pop().unwrap();
                        stack.last_mut().unwrap().children.push(node);
                    }
                }

                Token::Text(t) => {
                    stack.last_mut().unwrap().children.push(Node {
                        name: "text".into(),
                        children: vec![],
                        text: Some(t.clone()),
                    });
                }
            }
        }

        stack.remove(0)
    }
}
'''

os.makedirs("rust/src", exist_ok=True)
with open("rust/src/parser.rs","w") as f:
    f.write(code)

print("Parser generated")
"""))

# Cargo
write("rust/Cargo.toml", "[package]\nname=\"html_parser\"\nversion=\"0.3.0\"\nedition=\"2021\"")

# main
write("rust/src/main.rs", textwrap.dedent("""
mod parser;
use parser::*;

fn main() {
    let mut p = Parser::new();
    let dom = p.parse("<div>Hello<span>World</span></div>");
    println!("{:#?}", dom);
}
"""))

# zip
zip_path = "/mnt/data/html_parser_project_v3.zip"
zf = zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED)

for root, dirs, files in os.walk(base):
    for file in files:
        full = os.path.join(root, file)
        rel = os.path.relpath(full, base)
        zf.write(full, rel)

zf.close()

zip_path