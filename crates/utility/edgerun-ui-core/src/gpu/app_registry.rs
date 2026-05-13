use super::{UiAppKind, UiAppSurface, UiIcon};

pub const CHAT_APP_ID: u32 = 1;
pub const TRUST_MANAGER_APP_ID: u32 = 2;
pub const STORAGE_APP_ID: u32 = 3;
pub const COMPONENT_GALLERY_APP_ID: u32 = 5;
pub const LOCK_SCREEN_APP_ID: u32 = 10;
pub const CAPABILITY_REQUEST_APP_ID: u32 = 11;
pub const LAUNCH_CHAT_ITEM_ID: u32 = 881;
pub const LAUNCH_TRUST_MANAGER_ITEM_ID: u32 = 882;
pub const LAUNCH_STORAGE_ITEM_ID: u32 = 883;
pub const LAUNCH_COMPONENT_GALLERY_ITEM_ID: u32 = 884;
pub const LAUNCH_LOCK_SCREEN_ITEM_ID: u32 = 885;
pub const LAUNCH_CAPABILITY_REQUEST_ITEM_ID: u32 = 886;
pub const SHELL_LAUNCHER_BUTTON_ID: u32 = 870;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiAppPlacement {
    Workspace,
    FullScreenSystem,
    Example,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiAppSpec {
    pub id: u32,
    pub kind: UiAppKind,
    pub title: &'static str,
    pub detail: &'static str,
    pub icon: UiIcon,
    pub launch_id: u32,
    pub placement: UiAppPlacement,
}

pub const EDGERUN_APP_REGISTRY: &[UiAppSpec] = &[
    UiAppSpec {
        id: CHAT_APP_ID,
        kind: UiAppKind::Chat,
        title: "EdgeRun Chat",
        detail: "contacts, threads, encrypted messages",
        icon: UiIcon::Chat,
        launch_id: LAUNCH_CHAT_ITEM_ID,
        placement: UiAppPlacement::Workspace,
    },
    UiAppSpec {
        id: TRUST_MANAGER_APP_ID,
        kind: UiAppKind::TrustManager,
        title: "Trust Manager",
        detail: "identity, proofs, capability grants",
        icon: UiIcon::Trust,
        launch_id: LAUNCH_TRUST_MANAGER_ITEM_ID,
        placement: UiAppPlacement::Workspace,
    },
    UiAppSpec {
        id: STORAGE_APP_ID,
        kind: UiAppKind::Storage,
        title: "Storage",
        detail: "verified cache and payload objects",
        icon: UiIcon::Storage,
        launch_id: LAUNCH_STORAGE_ITEM_ID,
        placement: UiAppPlacement::Workspace,
    },
    UiAppSpec {
        id: COMPONENT_GALLERY_APP_ID,
        kind: UiAppKind::ComponentGallery,
        title: "Component Gallery",
        detail: "shared UI primitives",
        icon: UiIcon::App,
        launch_id: LAUNCH_COMPONENT_GALLERY_ITEM_ID,
        placement: UiAppPlacement::Example,
    },
    UiAppSpec {
        id: LOCK_SCREEN_APP_ID,
        kind: UiAppKind::LockScreen,
        title: "Lock Screen",
        detail: "Trust Container unlock",
        icon: UiIcon::Lock,
        launch_id: LAUNCH_LOCK_SCREEN_ITEM_ID,
        placement: UiAppPlacement::FullScreenSystem,
    },
    UiAppSpec {
        id: CAPABILITY_REQUEST_APP_ID,
        kind: UiAppKind::CapabilityRequest,
        title: "Capability Request",
        detail: "grant scoped app access",
        icon: UiIcon::Shield,
        launch_id: LAUNCH_CAPABILITY_REQUEST_ITEM_ID,
        placement: UiAppPlacement::FullScreenSystem,
    },
];

pub fn app_spec(kind: UiAppKind) -> Option<&'static UiAppSpec> {
    EDGERUN_APP_REGISTRY.iter().find(|spec| spec.kind == kind)
}

pub fn app_spec_for_launch_id(launch_id: u32) -> Option<&'static UiAppSpec> {
    EDGERUN_APP_REGISTRY
        .iter()
        .find(|spec| spec.launch_id == launch_id)
}

pub(super) fn app_id_for_kind(kind: UiAppKind) -> u32 {
    app_spec(kind).map(|spec| spec.id).unwrap_or(100)
}

pub(super) fn app_surface_for_kind(kind: UiAppKind) -> UiAppSurface {
    let Some(spec) = app_spec(kind) else {
        return UiAppSurface::new(100, "App");
    };
    UiAppSurface::new(spec.id, spec.title)
        .kind(spec.kind)
        .full_screen(spec.placement == UiAppPlacement::FullScreenSystem)
}
