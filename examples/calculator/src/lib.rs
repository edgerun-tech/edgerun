#![no_std]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use edgerun_sdk::host;
use edgerun_sdk::{Event, EventType, NetworkSubtype, render_jsx};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

static mut STATE: Option<CalcState> = None;

fn state() -> &'static mut CalcState {
    unsafe {
        if STATE.is_none() {
            STATE = Some(CalcState::new());
        }
        STATE.as_mut().unwrap()
    }
}

struct CalcState {
    display: String,
    prev: String,
    op: String,
    just_evaled: bool,
    history: Vec<String>,
}

impl CalcState {
    fn new() -> Self {
        Self {
            display: String::from("0"),
            prev: String::new(),
            op: String::new(),
            just_evaled: false,
            history: Vec::new(),
        }
    }

    fn digit(&mut self, d: &str) {
        if self.just_evaled {
            self.display = String::from(if d == "." { "0." } else { d });
            self.prev.clear();
            self.op.clear();
            self.just_evaled = false;
            return;
        }
        if d == "." && self.display.contains('.') {
            return;
        }
        if d != "." && self.display == "0" {
            self.display = String::from(d);
            return;
        }
        let digits = self.display.chars().filter(|c| c.is_ascii_digit() || *c == '.').count();
        if digits >= 12 {
            return;
        }
        self.display.push_str(d);
    }

    fn set_op(&mut self, op: &str) {
        if !self.op.is_empty() && !self.prev.is_empty() && !self.just_evaled {
            let result = self.eval();
            self.prev = result;
        } else {
            self.prev = self.display.clone();
        }
        self.op = String::from(op);
        self.just_evaled = false;
    }

    fn equals(&mut self) {
        if self.op.is_empty() || self.prev.is_empty() {
            self.just_evaled = true;
            return;
        }
        let result = self.eval();
        let entry = format!(
            "{} {} {} = {}",
            &self.prev,
            op_symbol(&self.op),
            &self.display,
            &result
        );
        self.history.insert(0, entry);
        if self.history.len() > 8 {
            self.history.truncate(8);
        }
        self.display = result;
        self.prev.clear();
        self.op.clear();
        self.just_evaled = true;
    }

    fn clear(&mut self) {
        *self = Self::new();
    }

    fn plus_minus(&mut self) {
        if self.display == "0" || self.display == "Error" {
            return;
        }
        if self.display.starts_with('-') {
            self.display = self.display[1..].to_string();
        } else {
            self.display = format!("-{}", self.display);
        }
    }

    fn percent(&mut self) {
        if let Ok(n) = self.display.parse::<f64>() {
            self.display = format!("{}", n / 100.0);
        }
    }

    fn backspace(&mut self) {
        if self.just_evaled || self.display == "Error" {
            self.display = String::from("0");
            self.just_evaled = false;
            return;
        }
        if self.display.len() <= 1 {
            self.display = String::from("0");
        } else {
            self.display.pop();
        }
    }

    fn eval(&self) -> String {
        let a = match self.prev.parse::<f64>() {
            Ok(v) => v,
            Err(_) => return self.display.clone(),
        };
        let b = match self.display.parse::<f64>() {
            Ok(v) => v,
            Err(_) => return self.display.clone(),
        };
        let result = match self.op.as_str() {
            "+" => a + b,
            "-" => a - b,
            "*" => a * b,
            "/" => {
                if b == 0.0 {
                    return String::from("Error");
                }
                a / b
            }
            _ => return self.display.clone(),
        };
        let formatted = format!("{:.10}", result)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string();
        if formatted.len() > 12 {
            format!("{:.5e}", result)
        } else {
            formatted
        }
    }

    fn handle_action(&mut self, action: &str) {
        let cmd = action.strip_prefix("action:").unwrap_or(action);
        match cmd {
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => self.digit(cmd),
            "dot" => self.digit("."),
            "add" => self.set_op("+"),
            "sub" => self.set_op("-"),
            "mul" => self.set_op("*"),
            "div" => self.set_op("/"),
            "eq" => self.equals(),
            "clear" => self.clear(),
            "plus_minus" => self.plus_minus(),
            "percent" => self.percent(),
            "backspace" => self.backspace(),
            _ => {}
        }
    }

    fn display_class(&self) -> &'static str {
        let len = self.display.len();
        if len > 10 {
            "text-2xl"
        } else if len > 7 {
            "text-3xl"
        } else {
            "text-4xl"
        }
    }

    fn to_jsx(&self) -> String {
        let display_text = if self.display == "Error" {
            String::from("Error")
        } else {
            let n = self.display.parse::<f64>().unwrap_or(0.0);
            if n.is_infinite() {
                String::from("Infinity")
            } else {
                format!("{:.10}", n)
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string()
            }
        };

        let op_label = op_symbol(&self.op);

        let mut html = String::new();
        html.push_str("<div class=\"flex h-full flex-col\">");

        // Display area
        html.push_str("<div class=\"flex-1 flex flex-col justify-end bg-[oklch(0.07_0.005_280)] px-5 py-4 min-h-0\">");

        // History lines
        html.push_str("<div class=\"mb-2 overflow-hidden\">");
        for (i, h) in self.history.iter().take(3).enumerate() {
            let opacity = 1.0 - (i as f64 * 0.3);
            html.push_str(&format!(
                "<p class=\"text-right font-mono text-[10px] text-muted-foreground/40 truncate\" style=\"opacity: {}\">{}</p>",
                opacity, escape_html(h)
            ));
        }
        html.push_str("</div>");

        // Sub-expression
        if !self.op.is_empty() && !self.prev.is_empty() {
            html.push_str(&format!(
                "<p class=\"text-right font-mono text-sm text-muted-foreground truncate\">{} {}</p>",
                escape_html(&self.prev),
                op_label
            ));
        }

        // Main display
        html.push_str(&format!(
            "<p class=\"text-right font-mono font-light tabular-nums text-foreground leading-none {}\">{}</p>",
            self.display_class(),
            escape_html(&display_text)
        ));

        html.push_str("</div>"); // close display area

        // Button grid
        html.push_str("<div class=\"grid grid-cols-4 gap-px bg-[var(--window-border)] border-t border-[var(--window-border)]\">");

        let buttons: Vec<(&str, &str, &str)> = vec![
            ("AC", "clear", "func"), ("+/−", "plus_minus", "func"), ("%", "percent", "func"), ("÷", "div", "op"),
            ("7", "7", "num"), ("8", "8", "num"), ("9", "9", "num"), ("×", "mul", "op"),
            ("4", "4", "num"), ("5", "5", "num"), ("6", "6", "num"), ("−", "sub", "op"),
            ("1", "1", "num"), ("2", "2", "num"), ("3", "3", "num"), ("+", "add", "op"),
            ("0", "0", "num"), (".", "dot", "num"), ("⌫", "backspace", "func"), ("=", "eq", "eq"),
        ];

        for (label, action, kind) in &buttons {
            let variant_class = match *kind {
                "op" => "bg-primary/20 text-primary hover:bg-primary/30 font-medium",
                "eq" => "bg-primary text-primary-foreground hover:opacity-90 font-semibold",
                "func" => "bg-secondary text-muted-foreground hover:bg-secondary/70 hover:text-foreground",
                _ => "bg-[oklch(0.16_0.005_280)] text-foreground hover:bg-[oklch(0.2_0.005_280)]",
            };

            let active_class = if *kind == "op" && !self.op.is_empty() && *label == op_label {
                "bg-primary text-primary-foreground"
            } else {
                ""
            };

            html.push_str(&format!(
                "<button data-action=\"action:{}\" class=\"flex h-14 items-center justify-center text-lg transition-all active:scale-95 {} {}\">{}</button>",
                action, variant_class, active_class, label
            ));
        }

        html.push_str("</div>"); // close grid
        html.push_str("</div>"); // close root

        html
    }
}

fn op_symbol(op: &str) -> String {
    match op {
        "+" => String::from("+"),
        "-" => String::from("−"),
        "*" => String::from("×"),
        "/" => String::from("÷"),
        _ => String::new(),
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn parse_action_from_event(event: &Event) -> Option<String> {
    if event.event_type != EventType::Network {
        return None;
    }
    if event.network_subtype()? != NetworkSubtype::Received {
        return None;
    }
    let payload = event.network_payload()?;
    let text = alloc::string::String::from_utf8(payload.to_vec()).ok()?;

    // Try JSON: {"action":"action:7"}
    if let Some(start) = text.find("\"action\":\"") {
        let rest = &text[start + 10..];
        if let Some(end) = rest.find('"') {
            return Some(String::from(&rest[..end]));
        }
    }
    // Try direct: action:7
    if text.starts_with("action:") {
        return Some(text);
    }
    Some(text)
}

#[no_mangle]
pub extern "C" fn run(_ptr: i32, _len: i32) -> i32 {
    // Check for incoming events
    let mut buf = [0u8; 1024];
    if let Some(event) = host::poll_event_safe(&mut buf) {
        if let Some(action) = parse_action_from_event(&event) {
            state().handle_action(&action);
        }
    }

    // Render UI as JSX
    let jsx = state().to_jsx();
    let resp = render_jsx(jsx);
    let output_bytes = resp.to_bytes();
    let output_ptr = output_bytes.as_ptr() as i32;
    let output_len = output_bytes.len() as i32;
    unsafe {
        host::write_output(output_ptr, output_len);
    }
    core::mem::forget(output_bytes);
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
