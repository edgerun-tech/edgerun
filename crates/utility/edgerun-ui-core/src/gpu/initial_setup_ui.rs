use super::{
    ButtonStyle, Color4, HitKind, UiAction, UiIcon, UiNode, button, card, column,
    progress_bar_node, row, shadcn_input, text,
};
use crate::initial_setup::PASSWORD_MIN_LEN;

pub const INITIAL_SETUP_PASSWORD_FIELD_ID: u32 = 91_001;
pub const INITIAL_SETUP_CONFIRM_FIELD_ID: u32 = 91_002;
pub const INITIAL_SETUP_CREATE_BUTTON_ID: u32 = 91_003;
pub const YUBIKEY_GRANT_PIN_FIELD_ID: u32 = 91_101;
pub const YUBIKEY_GRANT_SIGN_BUTTON_ID: u32 = 91_102;

const ACCENT: Color4 = Color4::rgba(0.090, 0.650, 0.530, 1.0);
const AMBER: Color4 = Color4::rgba(0.920, 0.660, 0.250, 1.0);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InitialSetupUiState {
    pub password: String,
    pub confirm_password: String,
    pub busy: bool,
    pub configured: bool,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InitialSetupUiIntent {
    None,
    CreatePasswordRoot { password: String },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct YubiKeyGrantCeremonyState {
    pub pin: String,
    pub busy: bool,
    pub signed: bool,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum YubiKeyGrantCeremonyIntent {
    None,
    SignGrant { pin: Option<String> },
}

impl InitialSetupUiState {
    pub fn handle_action(&mut self, action: &UiAction) -> InitialSetupUiIntent {
        match action {
            UiAction::TextChanged { id, value } if *id == INITIAL_SETUP_PASSWORD_FIELD_ID => {
                self.password = value.clone();
                self.status.clear();
                InitialSetupUiIntent::None
            }
            UiAction::TextChanged { id, value } if *id == INITIAL_SETUP_CONFIRM_FIELD_ID => {
                self.confirm_password = value.clone();
                self.status.clear();
                InitialSetupUiIntent::None
            }
            UiAction::Submitted { id }
                if *id == INITIAL_SETUP_PASSWORD_FIELD_ID
                    || *id == INITIAL_SETUP_CONFIRM_FIELD_ID =>
            {
                self.submit()
            }
            UiAction::Activated(hit)
                if hit.kind == HitKind::Button && hit.id == INITIAL_SETUP_CREATE_BUTTON_ID =>
            {
                self.submit()
            }
            _ => InitialSetupUiIntent::None,
        }
    }

    pub fn mark_configured(&mut self, envelope_len: usize) {
        self.password.clear();
        self.confirm_password.clear();
        self.busy = false;
        self.configured = true;
        self.status = format!("password root configured, {envelope_len} byte envelope");
    }

    pub fn mark_error(&mut self, message: impl Into<String>) {
        self.busy = false;
        self.status = message.into();
    }

    fn submit(&mut self) -> InitialSetupUiIntent {
        if self.busy {
            return InitialSetupUiIntent::None;
        }
        if self.password.len() < PASSWORD_MIN_LEN {
            self.status = format!("password must be at least {PASSWORD_MIN_LEN} bytes");
            return InitialSetupUiIntent::None;
        }
        if self.password != self.confirm_password {
            self.status = "passwords do not match".to_string();
            return InitialSetupUiIntent::None;
        }
        self.busy = true;
        InitialSetupUiIntent::CreatePasswordRoot {
            password: self.password.clone(),
        }
    }
}

impl YubiKeyGrantCeremonyState {
    pub fn handle_action(&mut self, action: &UiAction) -> YubiKeyGrantCeremonyIntent {
        match action {
            UiAction::TextChanged { id, value } if *id == YUBIKEY_GRANT_PIN_FIELD_ID => {
                self.pin = value
                    .chars()
                    .filter(|ch| ch.is_ascii_digit())
                    .take(8)
                    .collect();
                self.status.clear();
                YubiKeyGrantCeremonyIntent::None
            }
            UiAction::Submitted { id } if *id == YUBIKEY_GRANT_PIN_FIELD_ID => self.submit(),
            UiAction::Activated(hit)
                if hit.kind == HitKind::Button && hit.id == YUBIKEY_GRANT_SIGN_BUTTON_ID =>
            {
                self.submit()
            }
            _ => YubiKeyGrantCeremonyIntent::None,
        }
    }

    pub fn mark_signed(&mut self, summary: impl Into<String>) {
        self.pin.clear();
        self.busy = false;
        self.signed = true;
        self.status = summary.into();
    }

    pub fn mark_error(&mut self, message: impl Into<String>) {
        self.pin.clear();
        self.busy = false;
        self.status = message.into();
    }

    fn submit(&mut self) -> YubiKeyGrantCeremonyIntent {
        if self.busy {
            return YubiKeyGrantCeremonyIntent::None;
        }
        let pin = self.pin.trim();
        if !pin.is_empty() && !(6..=8).contains(&pin.len()) {
            self.pin.clear();
            self.status = "PIV PIN must be 6 to 8 digits".to_string();
            return YubiKeyGrantCeremonyIntent::None;
        }
        let submitted_pin = (!pin.is_empty()).then(|| pin.to_string());
        self.pin.clear();
        self.busy = true;
        YubiKeyGrantCeremonyIntent::SignGrant { pin: submitted_pin }
    }
}

pub fn build_initial_setup_surface(state: &InitialSetupUiState, _width: f32) -> UiNode {
    let progress = if state.configured { 1.0 } else { 0.15 };
    let status = if state.status.is_empty() {
        if state.configured {
            "configured"
        } else {
            "waiting for local secret"
        }
    } else {
        &state.status
    };
    column("w-full gap-6")
        .child(
            card("bg-panel border rounded-lg p-4 gap-3 h-28")
                .child(
                    row("h-8 w-full items-center justify-between gap-3")
                        .child(text("Initial Setup").class("h-5 text-text truncate"))
                        .child(UiNode::badge(
                            if state.configured {
                                "ready"
                            } else {
                                "required"
                            },
                            if state.configured { ACCENT } else { AMBER },
                        )),
                )
                .child(text(status).class("h-5 text-muted truncate"))
                .child(progress_bar_node(
                    progress,
                    if state.configured { ACCENT } else { AMBER },
                )),
        )
        .child(
            card("bg-panel border rounded-lg p-4 gap-4 h-92")
                .child(
                    row("h-12 w-full items-center gap-3")
                        .child(UiNode::icon(UiIcon::Lock).accent(ACCENT).class("size-6"))
                        .child(
                            column("flex-1 gap-1")
                                .child(text("Password Root").class("h-5 text-text truncate"))
                                .child(
                                    text("local secret material").class("h-5 text-muted truncate"),
                                ),
                        ),
                )
                .child(UiNode::divider("w-full"))
                .child(
                    shadcn_input("Password", &state.password)
                        .hit_id(INITIAL_SETUP_PASSWORD_FIELD_ID)
                        .masked(true)
                        .detail("minimum length enforced before setup"),
                )
                .child(
                    shadcn_input("Confirm password", &state.confirm_password)
                        .hit_id(INITIAL_SETUP_CONFIRM_FIELD_ID)
                        .masked(true)
                        .detail("must match the first password"),
                )
                .child(
                    row("h-12 w-full items-center justify-end").child(
                        button(
                            if state.busy { "Working" } else { "Create" },
                            INITIAL_SETUP_CREATE_BUTTON_ID,
                            ButtonStyle::Primary,
                        )
                        .class("h-10 w-28"),
                    ),
                ),
        )
}

pub fn build_yubikey_grant_ceremony_surface(state: &YubiKeyGrantCeremonyState) -> UiNode {
    let status = if state.status.is_empty() {
        if state.signed {
            "signed"
        } else {
            "waiting for YubiKey signature"
        }
    } else {
        &state.status
    };
    card("bg-panel border rounded-lg p-4 gap-4 h-76")
        .child(
            row("h-12 w-full items-center gap-3")
                .child(UiNode::icon(UiIcon::Key).accent(ACCENT).class("size-6"))
                .child(
                    column("flex-1 gap-1")
                        .child(text("YubiKey Grant Signature").class("h-5 text-text truncate"))
                        .child(text(status).class("h-5 text-muted truncate")),
                )
                .child(UiNode::badge(
                    if state.signed { "signed" } else { "required" },
                    if state.signed { ACCENT } else { AMBER },
                )),
        )
        .child(UiNode::divider("w-full"))
        .child(
            shadcn_input("PIV PIN", &state.pin)
                .hit_id(YUBIKEY_GRANT_PIN_FIELD_ID)
                .masked(true)
                .detail("leave empty only if the slot policy allows signing without PIN"),
        )
        .child(
            row("h-12 w-full items-center justify-end").child(
                button(
                    if state.busy { "Signing" } else { "Sign" },
                    YUBIKEY_GRANT_SIGN_BUTTON_ID,
                    ButtonStyle::Primary,
                )
                .class("h-10 w-28"),
            ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::{GpuScene, UiEvent, UiPainter, UiRect, UiRuntimeState};

    #[test]
    fn setup_state_validates_and_emits_create_intent() {
        let mut state = InitialSetupUiState::default();
        assert_eq!(
            state.handle_action(&UiAction::TextChanged {
                id: INITIAL_SETUP_PASSWORD_FIELD_ID,
                value: "correct horse battery".to_string(),
            }),
            InitialSetupUiIntent::None
        );
        assert_eq!(
            state.handle_action(&UiAction::TextChanged {
                id: INITIAL_SETUP_CONFIRM_FIELD_ID,
                value: "correct horse battery".to_string(),
            }),
            InitialSetupUiIntent::None
        );
        assert_eq!(
            state.handle_action(&UiAction::Activated(super::super::GpuHit::new(
                HitKind::Button,
                INITIAL_SETUP_CREATE_BUTTON_ID,
                0.0,
                0.0,
                10.0,
                10.0,
            ))),
            InitialSetupUiIntent::CreatePasswordRoot {
                password: "correct horse battery".to_string(),
            }
        );
    }

    #[test]
    fn setup_surface_exposes_masked_fields_and_button() {
        let state = InitialSetupUiState::default();
        let layout = build_initial_setup_surface(&state, 640.0);
        let mut scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 1.0));
        let mut runtime = UiRuntimeState::default();
        {
            let mut ui = UiPainter::new(&mut scene);
            layout.render_with_state(&mut ui, UiRect::new(0.0, 0.0, 640.0, 520.0), Some(&runtime));
        }
        assert!(scene.hits().iter().any(|hit| {
            hit.kind == HitKind::Input && hit.id == INITIAL_SETUP_PASSWORD_FIELD_ID
        }));
        assert!(scene.hits().iter().any(|hit| {
            hit.kind == HitKind::Button && hit.id == INITIAL_SETUP_CREATE_BUTTON_ID
        }));

        let hit = scene
            .hits()
            .iter()
            .copied()
            .find(|hit| hit.kind == HitKind::Input && hit.id == INITIAL_SETUP_PASSWORD_FIELD_ID)
            .expect("password field hit");
        assert!(matches!(
            runtime.handle_event(
                &scene,
                UiEvent::PointerDown {
                    x: hit.x + 2.0,
                    y: hit.y + 2.0
                }
            ),
            UiAction::Focused(_)
        ));
        assert_eq!(
            runtime.handle_event(&scene, UiEvent::TextInput("secret".to_string())),
            UiAction::TextChanged {
                id: INITIAL_SETUP_PASSWORD_FIELD_ID,
                value: "secret".to_string(),
            }
        );
    }

    #[test]
    fn yubikey_grant_ceremony_validates_pin_and_emits_sign_intent() {
        let mut state = YubiKeyGrantCeremonyState::default();
        assert_eq!(
            state.handle_action(&UiAction::TextChanged {
                id: YUBIKEY_GRANT_PIN_FIELD_ID,
                value: "12345".to_string(),
            }),
            YubiKeyGrantCeremonyIntent::None
        );
        assert_eq!(
            state.handle_action(&UiAction::Activated(super::super::GpuHit::new(
                HitKind::Button,
                YUBIKEY_GRANT_SIGN_BUTTON_ID,
                0.0,
                0.0,
                10.0,
                10.0,
            ))),
            YubiKeyGrantCeremonyIntent::None
        );
        assert!(state.status.contains("6 to 8"));

        assert_eq!(
            state.handle_action(&UiAction::TextChanged {
                id: YUBIKEY_GRANT_PIN_FIELD_ID,
                value: "123456".to_string(),
            }),
            YubiKeyGrantCeremonyIntent::None
        );
        assert_eq!(
            state.handle_action(&UiAction::Activated(super::super::GpuHit::new(
                HitKind::Button,
                YUBIKEY_GRANT_SIGN_BUTTON_ID,
                0.0,
                0.0,
                10.0,
                10.0,
            ))),
            YubiKeyGrantCeremonyIntent::SignGrant {
                pin: Some("123456".to_string())
            }
        );
    }
}
