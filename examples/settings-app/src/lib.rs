#![no_std]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use edgerun_sdk::{Request, Response, host, render, ui, UINode};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(Debug, Clone, PartialEq)]
enum Screen {
    Welcome,
    Identity,
    Controllers,
    Networking,
    Complete,
}

struct AppState {
    screen: Screen,
    has_identity: bool,
    identity_label: String,
    controller_count: u32,
    bootstrap_complete: bool,
    input_buffer: String,
    status_message: String,
}

impl AppState {
    fn new() -> Self {
        Self {
            screen: Screen::Welcome,
            has_identity: false,
            identity_label: String::new(),
            controller_count: 0,
            bootstrap_complete: false,
            input_buffer: String::new(),
            status_message: String::new(),
        }
    }

    fn handle_action(&mut self, action: &str) -> Vec<(&'static str, Vec<u8>)> {
        let mut emitted_commands: Vec<(&'static str, Vec<u8>)> = Vec::new();

        if action.starts_with("action:") {
            let cmd = &action[7..];
            match cmd {
                "start_setup" => self.screen = Screen::Identity,
                "generate_identity" => {
                    self.has_identity = true;
                    self.identity_label = String::from("primary");
                    self.status_message = String::from("Identity created");
                    self.screen = Screen::Controllers;
                    emitted_commands.push(("create_identity", Vec::new()));
                }
                "import_identity" => {
                    self.has_identity = true;
                    self.identity_label = String::from("imported");
                    self.status_message = String::from("Identity imported");
                    self.screen = Screen::Controllers;
                    emitted_commands.push(("import_identity", Vec::new()));
                }
                "add_controller" => {
                    self.controller_count += 1;
                    self.status_message = String::from("Controller added");
                    if self.has_identity && self.controller_count > 0 {
                        self.bootstrap_complete = true;
                        self.screen = Screen::Complete;
                    }
                    emitted_commands.push(("add_controller", Vec::new()));
                }
                "add_bootstrap_peer" => {
                    self.status_message = String::from("Bootstrap peer added");
                    emitted_commands.push(("add_bootstrap_peer", Vec::new()));
                }
                "back" => match self.screen {
                    Screen::Identity => self.screen = Screen::Welcome,
                    Screen::Controllers => self.screen = Screen::Identity,
                    Screen::Networking => self.screen = Screen::Controllers,
                    _ => {}
                },
                "next" => match self.screen {
                    Screen::Identity => self.screen = Screen::Controllers,
                    Screen::Controllers => {
                        if self.has_identity && self.controller_count > 0 {
                            self.bootstrap_complete = true;
                            self.screen = Screen::Complete;
                        } else {
                            self.screen = Screen::Networking;
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        emitted_commands
    }

    fn to_ui(&self) -> UINode {
        match &self.screen {
            Screen::Welcome => self.ui_welcome(),
            Screen::Identity => self.ui_identity(),
            Screen::Controllers => self.ui_controllers(),
            Screen::Networking => self.ui_networking(),
            Screen::Complete => self.ui_complete(),
        }
    }

    fn ui_welcome(&self) -> UINode {
        ui::column(vec![
            ui::heading("EdgeRun Node Setup", 1),
            ui::spacer(16),
            ui::text("Welcome! This app will help you configure your new EdgeRun node."),
            ui::spacer(12),
            ui::text("You will set up:"),
            ui::text("  - Node identity"),
            ui::text("  - Controller access"),
            ui::text("  - Network peers"),
            ui::spacer(16),
            ui::button("Start Setup", "start_setup"),
        ])
    }

    fn ui_identity(&self) -> UINode {
        let mut children = vec![
            ui::heading("Node Identity", 2),
            ui::spacer(12),
            ui::text("Your node needs a cryptographic identity."),
            ui::spacer(8),
        ];

        if self.has_identity {
            children.push(ui::text(&alloc::format!("Identity: {} (configured)", self.identity_label)));
            children.push(ui::spacer(4));
            children.push(ui::text(&self.status_message));
        } else {
            children.push(ui::button("Generate New Identity", "generate_identity"));
            children.push(ui::spacer(4));
            children.push(ui::button("Import Existing Identity", "import_identity"));
        }

        children.push(ui::spacer(12));
        children.push(ui::row(vec![
            ui::button("Back", "back"),
            ui::button("Next", "next"),
        ]));

        ui::column(children)
    }

    fn ui_controllers(&self) -> UINode {
        let mut children = vec![
            ui::heading("Controllers", 2),
            ui::spacer(12),
            ui::text("Controllers are identities that can manage this node."),
            ui::spacer(8),
        ];

        children.push(ui::text(&alloc::format!("Configured controllers: {}", self.controller_count)));
        children.push(ui::spacer(4));

        if !self.status_message.is_empty() {
            children.push(ui::text(&self.status_message));
            children.push(ui::spacer(4));
        }

        children.push(ui::button("Add Controller", "add_controller"));
        children.push(ui::spacer(12));
        children.push(ui::row(vec![
            ui::button("Back", "back"),
            ui::button("Next", "next"),
        ]));

        ui::column(children)
    }

    fn ui_networking(&self) -> UINode {
        let mut children = vec![
            ui::heading("Network Peers", 2),
            ui::spacer(12),
            ui::text("Add bootstrap peers for mesh discovery."),
            ui::spacer(8),
        ];

        if !self.status_message.is_empty() {
            children.push(ui::text(&self.status_message));
            children.push(ui::spacer(4));
        }

        children.push(ui::button("Add Bootstrap Peer", "add_bootstrap_peer"));
        children.push(ui::spacer(12));
        children.push(ui::row(vec![
            ui::button("Back", "back"),
            ui::button("Finish", "next"),
        ]));

        ui::column(children)
    }

    fn ui_complete(&self) -> UINode {
        ui::column(vec![
            ui::heading("Setup Complete", 1),
            ui::spacer(16),
            ui::text("Your node is now configured!"),
            ui::spacer(8),
            ui::text(&alloc::format!("Identity: {}", self.identity_label)),
            ui::text(&alloc::format!("Controllers: {}", self.controller_count)),
            ui::spacer(12),
            ui::text("The settings app will now close and normal execution will begin."),
            ui::spacer(16),
            ui::text("Status: READY"),
        ])
    }
}

#[edgerun_sdk::main]
fn handle(req: Request) -> Response {
    let mut state = AppState::new();

    let body = req.body_as_string().unwrap_or_default();
    if !body.is_empty() {
        let commands = state.handle_action(&body);
        for (cmd_name, _payload) in commands {
            let target = "";
            let payload = cmd_name.as_bytes();
            host::send_message_safe(target, payload);
        }
    }

    render(state.to_ui())
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
