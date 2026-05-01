#![no_std]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec;
use edgerun_sdk::{Request, Response, render, ui};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

struct State {
    count: i32,
}

impl State {
    fn new() -> Self {
        Self { count: 0 }
    }

    fn handle_action(&mut self, action: &str) {
        if action.starts_with("action:") {
            let cmd = &action[7..];
            match cmd {
                "increment" => self.count += 1,
                "decrement" => self.count -= 1,
                "reset" => self.count = 0,
                _ => {}
            }
        }
    }

    fn to_ui(&self) -> edgerun_sdk::UINode {
        let count_str = self.count.to_string();

        ui::column(vec![
            ui::heading("Counter", 1),
            ui::spacer(12),
            ui::text(&count_str),
            ui::spacer(8),
            ui::row(vec![
                ui::button("- 1", "decrement"),
                ui::button("+ 1", "increment"),
            ]),
            ui::spacer(4),
            ui::button("Reset", "reset"),
        ])
    }
}

#[edgerun_sdk::main]
fn handle(req: Request) -> Response {
    let mut state = State::new();

    let body = req.body_as_string().unwrap_or_default();
    if !body.is_empty() {
        let action = parse_action(&body);
        state.handle_action(&action);
    }

    render(state.to_ui())
}

fn parse_action(input: &str) -> String {
    if input.starts_with("{\"action\":\"") {
        if let Some(end) = input[12..].find('"') {
            return String::from(&input[12..12 + end]);
        }
    }
    if input.starts_with("action:") {
        return String::from(input);
    }
    String::from(input)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
