use simple_project::{Config, process};

fn main() {
    let config = Config::new("cli");
    let result = process("hello world", &config);
    println!("{}", result);
}
