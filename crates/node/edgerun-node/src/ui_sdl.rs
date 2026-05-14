use std::cell::RefCell;
use std::format;
use std::string::{String, ToString};
use std::time::{SystemTime, UNIX_EPOCH};
use std::vec;
use std::vec::Vec;

use edgerun_ui_core::gpu::sdl::{
    SDL_KEY_ESCAPE, SdlEventResult, SdlGlWindowOptions, SdlInputEvent, run_sdl_gl_window,
};
use edgerun_ui_core::gpu::{ButtonStyle, Color4, HitKind, UiAction, UiIcon, UiNode};

use crate::capacity::{NodeCapacity, format_bytes};
use crate::hardware::HardwareInventory;

const SDLK_R: i32 = 114;

const BG: Color4 = Color4::rgba(0.030, 0.034, 0.041, 1.0);
const ACCENT: Color4 = Color4::rgba(0.090, 0.650, 0.530, 1.0);
const BLUE: Color4 = Color4::rgba(0.280, 0.560, 0.920, 1.0);
const AMBER: Color4 = Color4::rgba(0.920, 0.660, 0.250, 1.0);
const NAV_MACHINE_REPORT_ID: u32 = 10_001;
const NAV_NODE_INSTANCES_ID: u32 = 10_002;
const NAV_STORAGE_ID: u32 = 10_003;
const NAV_TRUST_ID: u32 = 10_004;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActivePanel {
    MachineReport,
    NodeInstances,
    Storage,
    Trust,
}

impl ActivePanel {
    const fn title(self) -> &'static str {
        match self {
            Self::MachineReport => "Machine Report",
            Self::NodeInstances => "Node Instances",
            Self::Storage => "Storage",
            Self::Trust => "Trust",
        }
    }

    const fn subtitle(self) -> &'static str {
        match self {
            Self::MachineReport => "local hardware, capacity, and providers",
            Self::NodeInstances => "role instances, runtimes, and local authority",
            Self::Storage => "content, cache, and object materialization",
            Self::Trust => "identity, policy, and proof state",
        }
    }
}

#[derive(Clone, Debug)]
struct ReportSection {
    title: &'static str,
    detail: String,
    accent: Color4,
    rows: Vec<String>,
}

#[derive(Clone, Debug)]
struct MachineReport {
    refreshed_at: String,
    platform: &'static str,
    cores: String,
    memory: String,
    capability_count: usize,
    device_count: usize,
    sections: Vec<ReportSection>,
}

pub fn run_window() -> Result<(), String> {
    run_window_for_frames(None)
}

pub fn run_window_for_frames(frames: Option<u32>) -> Result<(), String> {
    let state = RefCell::new(NodeUiState {
        report: MachineReport::collect(),
        active_panel: ActivePanel::MachineReport,
        scroll_y: 0.0,
    });
    run_sdl_gl_window(
        SdlGlWindowOptions::new("EdgeRun Node", 1180, 760, BG)
            .min_size(760, 520)
            .frames(frames),
        |event| match event {
            SdlInputEvent::CloseRequested => SdlEventResult::quit(),
            SdlInputEvent::KeyDown {
                key: SDL_KEY_ESCAPE,
            } => SdlEventResult::quit(),
            SdlInputEvent::KeyDown { key } if key == SDLK_R || key == SDLK_R - 32 => {
                state.borrow_mut().report = MachineReport::collect();
                SdlEventResult::dirty()
            }
            SdlInputEvent::UiAction {
                action: UiAction::Activated(hit),
            } if hit.kind == HitKind::MenuItem => {
                let mut state = state.borrow_mut();
                let Some(active_panel) = nav_panel_for_id(hit.id) else {
                    return SdlEventResult::default();
                };
                if state.active_panel != active_panel {
                    state.active_panel = active_panel;
                    state.scroll_y = 0.0;
                    SdlEventResult::dirty()
                } else {
                    SdlEventResult::default()
                }
            }
            SdlInputEvent::MouseWheel { y } => {
                let mut state = state.borrow_mut();
                state.scroll_y = (state.scroll_y - y * 52.0).max(0.0);
                SdlEventResult::dirty()
            }
            SdlInputEvent::Resized { .. } => SdlEventResult::dirty(),
            _ => SdlEventResult::default(),
        },
        |width, height| {
            let state = state.borrow();
            build_layout(
                &state.report,
                state.active_panel,
                width as f32,
                height as f32,
                state.scroll_y,
            )
        },
    )
}

struct NodeUiState {
    report: MachineReport,
    active_panel: ActivePanel,
    scroll_y: f32,
}

impl MachineReport {
    fn collect() -> Self {
        let inventory = HardwareInventory::discover();
        let capacity = NodeCapacity::discover();
        let sections = section_list(&inventory);
        let device_count = sections.iter().map(|section| section.rows.len()).sum();

        Self {
            refreshed_at: refresh_label(),
            platform: inventory.platform,
            cores: capacity.total_cores.to_string(),
            memory: format_bytes(capacity.total_memory_bytes),
            capability_count: inventory.capability_descriptors.len(),
            device_count,
            sections,
        }
    }
}

fn section_list(inventory: &HardwareInventory) -> Vec<ReportSection> {
    vec![
        section(
            "Compute",
            "GPU and accelerator devices",
            ACCENT,
            merge_rows(&[&inventory.gpus, &inventory.npu_devices]),
        ),
        section(
            "Displays",
            "DRM connectors and output state",
            BLUE,
            inventory.displays.clone(),
        ),
        section(
            "Network",
            "WiFi, Bluetooth, NFC, and CEC",
            ACCENT,
            merge_rows(&[
                &inventory.wifi_interfaces,
                &inventory.bluetooth_controllers,
                &inventory.nfc_adapters,
                &inventory.cec_adapters,
            ]),
        ),
        section(
            "Input",
            "Human input and biometric devices",
            AMBER,
            merge_rows(&[
                &inventory.input_devices,
                &inventory.fingerprint_readers,
                &inventory.biometric,
            ]),
        ),
        section(
            "Media",
            "Audio capture, output, and camera",
            BLUE,
            merge_rows(&[
                &inventory.audio_input,
                &inventory.audio_output,
                &inventory.camera,
                &inventory.sensors,
            ]),
        ),
        section(
            "Buses",
            "USB and PCI inventory",
            ACCENT,
            merge_rows(&[&inventory.usb_devices, &inventory.pci_devices]),
        ),
        section(
            "Power",
            "Battery and power supply state",
            AMBER,
            inventory.power_supplies.clone(),
        ),
        section(
            "Authority",
            "Keystore and capability providers",
            ACCENT,
            merge_rows(&[&inventory.keystore, &inventory.location]),
        ),
    ]
}

fn section(title: &'static str, detail: &str, accent: Color4, rows: Vec<String>) -> ReportSection {
    ReportSection {
        title,
        detail: detail.to_string(),
        accent,
        rows,
    }
}

fn merge_rows(groups: &[&Vec<String>]) -> Vec<String> {
    let mut rows = Vec::new();
    for group in groups {
        rows.extend(group.iter().cloned());
    }
    rows
}

fn build_layout(
    report: &MachineReport,
    active_panel: ActivePanel,
    width: f32,
    _height: f32,
    scroll_y: f32,
) -> UiNode {
    let content_w = (width - 248.0).max(420.0);
    UiNode::row("w-full h-full bg-bg")
        .child(
            sidebar_layout(report, active_panel).class("w-62 h-full bg-sidebar border p-6 gap-4"),
        )
        .child(
            UiNode::column("flex-1 h-full bg-bg")
                .child(
                    header_layout(report, active_panel)
                        .class("h-19 w-full bg-topbar border px-7 py-4"),
                )
                .child(
                    UiNode::scroll_area_px("flex-1 w-full overflow-hidden p-7 gap-6", scroll_y)
                        .child(panel_body(report, active_panel, content_w))
                        .child(
                            UiNode::text("Press R to refresh. Esc closes the native node UI.")
                                .class("h-6 text-muted"),
                        ),
                ),
        )
}

fn sidebar_layout(report: &MachineReport, active_panel: ActivePanel) -> UiNode {
    UiNode::column("w-full h-full gap-5")
        .child(
            UiNode::row("h-12 w-full items-center gap-3")
                .child(UiNode::icon(UiIcon::Server).class("size-7 text-accent"))
                .child(
                    UiNode::column("flex-1 gap-1")
                        .child(UiNode::text("edgerun node").class("h-5 text-text truncate"))
                        .child(UiNode::text("machine authority").class("h-5 text-muted truncate")),
                ),
        )
        .child(UiNode::divider("w-full"))
        .child(nav_item(
            "Machine report",
            "hardware and providers",
            NAV_MACHINE_REPORT_ID,
            active_panel == ActivePanel::MachineReport,
        ))
        .child(nav_item(
            "Node instances",
            "roles and runtimes",
            NAV_NODE_INSTANCES_ID,
            active_panel == ActivePanel::NodeInstances,
        ))
        .child(nav_item(
            "Storage",
            "objects and cache",
            NAV_STORAGE_ID,
            active_panel == ActivePanel::Storage,
        ))
        .child(nav_item(
            "Trust",
            "policy and proofs",
            NAV_TRUST_ID,
            active_panel == ActivePanel::Trust,
        ))
        .child(UiNode::spacer("flex-1"))
        .child(UiNode::divider("w-full"))
        .child(summary_row("platform", report.platform))
        .child(summary_row("devices", &report.device_count.to_string()))
        .child(summary_row(
            "providers",
            &report.capability_count.to_string(),
        ))
}

fn nav_panel_for_id(id: u32) -> Option<ActivePanel> {
    match id {
        NAV_MACHINE_REPORT_ID => Some(ActivePanel::MachineReport),
        NAV_NODE_INSTANCES_ID => Some(ActivePanel::NodeInstances),
        NAV_STORAGE_ID => Some(ActivePanel::Storage),
        NAV_TRUST_ID => Some(ActivePanel::Trust),
        _ => None,
    }
}

fn nav_item(label: &str, detail: &str, id: u32, active: bool) -> UiNode {
    UiNode::menu_item(label, id)
        .detail(detail)
        .accent(if active { ACCENT } else { BLUE })
        .selected(active)
        .class("h-12 w-full")
}

fn summary_row(label: &str, value: &str) -> UiNode {
    UiNode::row("h-7 w-full items-center justify-between gap-2")
        .child(UiNode::text(label).class("h-5 text-muted truncate"))
        .child(UiNode::text(value).class("h-5 text-text truncate"))
}

fn header_layout(report: &MachineReport, active_panel: ActivePanel) -> UiNode {
    UiNode::row("w-full h-full items-center justify-between gap-4")
        .child(
            UiNode::column("flex-1 gap-1")
                .child(UiNode::text(active_panel.title()).class("h-5 text-text truncate"))
                .child(UiNode::text(active_panel.subtitle()).class("h-5 text-muted truncate")),
        )
        .child(UiNode::badge("SDL native", BLUE).class("h-6"))
        .child(UiNode::badge(&report.refreshed_at, ACCENT).class("h-6"))
}

fn panel_body(report: &MachineReport, active_panel: ActivePanel, width: f32) -> UiNode {
    match active_panel {
        ActivePanel::MachineReport => machine_report_panel(report, width),
        ActivePanel::NodeInstances => node_instances_panel(report, width),
        ActivePanel::Storage => storage_panel(report, width),
        ActivePanel::Trust => trust_panel(report, width),
    }
}

fn machine_report_panel(report: &MachineReport, width: f32) -> UiNode {
    UiNode::column("w-full gap-6")
        .child(metric_grid_layout(report, width))
        .child(section_grid_layout(report, width))
}

fn node_instances_panel(report: &MachineReport, width: f32) -> UiNode {
    UiNode::column("w-full gap-6")
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-3 gap-4", width)
                .class("w-full")
                .child(
                    metric_node("Admission", "local", "policy gate for this device", ACCENT)
                        .progress(0.25),
                )
                .child(metric_node(
                    "Relay",
                    "not set",
                    "assigned packet path",
                    AMBER,
                ))
                .child(metric_node(
                    "Storage",
                    "not set",
                    "content-addressed cache",
                    BLUE,
                )),
        )
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-4", width)
                .class("w-full")
                .child(control_card(
                    "Role Instances",
                    "Native and browser-capable roles that this device can run.",
                    UiIcon::Server,
                    ACCENT,
                    &[
                        control_value("admission:device", "local policy authority", "planned"),
                        control_value("relay:private-devices", "device mesh ingress", "not set"),
                        control_value(
                            "storage:local-cache",
                            "verified package/object cache",
                            "not set",
                        ),
                        control_value("notary:device", "TPM-backed sealing boundary", "planned"),
                    ],
                ))
                .child(control_card(
                    "Hardware Providers",
                    "Discovered local capabilities that can back node roles.",
                    UiIcon::Cpu,
                    BLUE,
                    &[
                        control_value(
                            "hardware devices",
                            "from machine report",
                            &report.device_count.to_string(),
                        ),
                        control_value(
                            "capability providers",
                            "runtime descriptors",
                            &report.capability_count.to_string(),
                        ),
                        control_value("platform", "native target", report.platform),
                    ],
                )),
        )
}

fn storage_panel(report: &MachineReport, width: f32) -> UiNode {
    UiNode::column("w-full gap-6")
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-4 gap-4", width)
                .class("w-full")
                .child(metric_node("CAS", "not set", "object root", AMBER))
                .child(metric_node("Cache", "unknown", "verified bytes", BLUE))
                .child(metric_node(
                    "Relay Route",
                    "required",
                    "all movement goes through relay",
                    ACCENT,
                ))
                .child(metric_node(
                    "Materialize",
                    "manual",
                    "object to filesystem",
                    BLUE,
                )),
        )
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-4", width)
                .class("w-full")
                .child(control_card(
                    "Storage Nodes",
                    "Configure where packets, sealed objects, and verified caches live.",
                    UiIcon::Storage,
                    BLUE,
                    &[
                        control_button(
                            "Add local cache path",
                            "choose folder for verified objects",
                            20_101,
                        ),
                        control_button(
                            "Attach network storage",
                            "request admission before sending packets",
                            20_102,
                        ),
                        control_value(
                            "sealing boundary",
                            "VFS must not hold sealing keys",
                            "notary",
                        ),
                    ],
                ))
                .child(control_card(
                    "Object Flow",
                    "Content should stay addressable without forcing plaintext assembly.",
                    UiIcon::Route,
                    ACCENT,
                    &[
                        control_value(
                            "object to packets",
                            "BLAKE3-addressed packet stream",
                            "planned",
                        ),
                        control_value(
                            "packets to object",
                            "reassemble after relay delivery",
                            "planned",
                        ),
                        control_value(
                            "object to file",
                            "materialize only on explicit request",
                            "planned",
                        ),
                    ],
                )),
        )
        .child(section_subset_card(
            report,
            "Buses",
            "local storage and USB-adjacent hardware",
            BLUE,
        ))
}

fn trust_panel(report: &MachineReport, width: f32) -> UiNode {
    UiNode::column("w-full gap-6")
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-4 gap-4", width)
                .class("w-full")
                .child(metric_node("Admission", "not configured", "device policy authority", AMBER))
                .child(metric_node("Device Key", "not sealed", "TPM-backed node identity", AMBER))
                .child(metric_node("Master Key", "locked", "sealed Trust Container root", BLUE))
                .child(metric_node("Proof Trail", "local", "audit events from this device", ACCENT)),
        )
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-4", width)
                .class("w-full")
                .child(control_card(
                    "Admission Authority",
                    "Choose the policy gate this device uses before work enters the network.",
                    UiIcon::Shield,
                    ACCENT,
                    &[
                        control_button("Use EdgeRun DAO admission", "default public admission path", 21_001),
                        control_button("Create device admission", "local policy for this machine", 21_002),
                        control_button("Attach owned admission", "use a user or organization authority", 21_003),
                        control_value("route policy", "relay/channel assignment required", "strict"),
                    ],
                ))
                .child(control_card(
                    "Device Key",
                    "Bind this native node identity to hardware before admitting local roles.",
                    UiIcon::Key,
                    BLUE,
                    &[
                        control_button("Create TPM device key", "derive node identity from hardware key", 21_101),
                        control_button("Enroll YubiKey unlock", "require security key for release", 21_102),
                        control_button("Enroll fingerprint unlock", "local biometric gate when supported", 21_103),
                        control_value("node identity", "must match derive_node_id(public_key, role)", "pending"),
                    ],
                )),
        )
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-4", width)
                .class("w-full")
                .child(control_card(
                    "Trust Container",
                    "Seal the user master key to device policy and explicit unlock factors.",
                    UiIcon::Lock,
                    AMBER,
                    &[
                        control_button("Seal user master key", "TPM NVRAM plus recovery policy", 21_201),
                        control_button("Test unseal", "fingerprint, YubiKey, or password", 21_202),
                        control_value("browser node handoff", "WASM node should ask host notary to unseal", "planned"),
                    ],
                ))
                .child(control_card(
                    "Proof Dashboard",
                    "Every admission, unseal, relay setup, and storage grant should leave evidence.",
                    UiIcon::Trust,
                    ACCENT,
                    &[
                        control_value("admission policy hash", "content-addressed policy commitment", "unknown"),
                        control_value("delivery reports", "notary signs plaintext release events", "planned"),
                        control_value("relay/storage grants", "show accepted route authority", "pending"),
                    ],
                )),
        )
        .child(section_subset_card(report, "Authority", "detected local trust providers", ACCENT))
        .child(section_subset_card(report, "Input", "possible unlock factors on this device", AMBER))
}

fn metric_grid_layout(report: &MachineReport, width: f32) -> UiNode {
    UiNode::grid_auto_for_width(
        "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4",
        width,
    )
    .class("w-full")
    .child(metric_node("CPU", &report.cores, "logical cores", ACCENT))
    .child(metric_node("Memory", &report.memory, "physical RAM", BLUE))
    .child(metric_node(
        "Devices",
        &report.device_count.to_string(),
        "discovered entries",
        AMBER,
    ))
    .child(metric_node(
        "Providers",
        &report.capability_count.to_string(),
        "capability descriptors",
        ACCENT,
    ))
}

fn metric_node(title: &str, value: &str, detail: &str, accent: Color4) -> UiNode {
    UiNode::metric_card(title, value)
        .detail(detail)
        .accent(accent)
        .class("h-32")
}

#[derive(Clone, Copy)]
enum ControlAccessorySpec<'a> {
    Value(&'a str),
    Button(&'a str, u32),
}

#[derive(Clone, Copy)]
struct ControlSpec<'a> {
    label: &'a str,
    detail: &'a str,
    accessory: ControlAccessorySpec<'a>,
}

fn control_value<'a>(label: &'a str, detail: &'a str, value: &'a str) -> ControlSpec<'a> {
    ControlSpec {
        label,
        detail,
        accessory: ControlAccessorySpec::Value(value),
    }
}

fn control_button<'a>(label: &'a str, detail: &'a str, id: u32) -> ControlSpec<'a> {
    ControlSpec {
        label,
        detail,
        accessory: ControlAccessorySpec::Button("Set up", id),
    }
}

fn control_card(
    title: &str,
    detail: &str,
    icon: UiIcon,
    accent: Color4,
    rows: &[ControlSpec<'_>],
) -> UiNode {
    let height_units = 32 + rows.len().max(1) * 15;
    let class = format!("bg-panel border rounded-lg overflow-hidden p-4 gap-3 h-{height_units}");
    rows.iter().fold(
        UiNode::card(&class)
            .child(
                UiNode::row("h-12 w-full items-center gap-3")
                    .child(UiNode::icon(icon).accent(accent).class("size-6"))
                    .child(
                        UiNode::column("flex-1 gap-1")
                            .child(UiNode::text(title).class("h-5 text-text truncate"))
                            .child(UiNode::text(detail).class("h-5 text-muted truncate")),
                    ),
            )
            .child(UiNode::divider("w-full")),
        |node, row| node.child(control_row_node(row)),
    )
}

fn control_row_node(row: &ControlSpec<'_>) -> UiNode {
    let node = UiNode::control_row(row.label)
        .detail(row.detail)
        .class("h-12 w-full rounded-sm bg-row");
    match row.accessory {
        ControlAccessorySpec::Value(value) => node.value_text(value),
        ControlAccessorySpec::Button(label, id) => {
            node.control_button(label, id, ButtonStyle::Secondary)
        }
    }
}

fn section_subset_card(
    report: &MachineReport,
    title: &'static str,
    detail: &str,
    accent: Color4,
) -> UiNode {
    report
        .sections
        .iter()
        .find(|section| section.title == title)
        .map(section_node)
        .unwrap_or_else(|| {
            let fallback = section(title, detail, accent, vec!["none discovered".to_string()]);
            section_node(&fallback)
        })
}

fn section_grid_layout(report: &MachineReport, width: f32) -> UiNode {
    report.sections.iter().fold(
        UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-4", width).class("w-full"),
        |node, section| node.child(section_node(section)),
    )
}

fn section_node(section: &ReportSection) -> UiNode {
    let visible_rows = section.rows.len().min(6);
    let height_units = 27 + visible_rows.max(1) * 12;
    let class = format!("bg-panel border rounded-lg overflow-hidden p-4 gap-3 h-{height_units}");
    let mut node = UiNode::card(&class)
        .child(
            UiNode::row("h-12 w-full items-center gap-3")
                .child(
                    UiNode::column("flex-1 gap-1")
                        .child(UiNode::text(section.title).class("h-5 text-text truncate"))
                        .child(UiNode::text(&section.detail).class("h-5 text-muted truncate")),
                )
                .child(UiNode::badge(
                    &section.rows.len().to_string(),
                    section.accent,
                )),
        )
        .child(UiNode::divider("w-full"));

    if section.rows.is_empty() {
        node = node.child(UiNode::text("none discovered").class("h-6 text-muted"));
    } else {
        for (index, row) in section.rows.iter().take(visible_rows).enumerate() {
            node = node.child(
                UiNode::list_row(row, "", 30_000 + index as u32)
                    .accent(section.accent)
                    .class("h-9 rounded-sm bg-row"),
            );
        }
    }
    node
}

fn refresh_label() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() % 86_400)
        .unwrap_or(0);
    let hour = seconds / 3600;
    let minute = (seconds % 3600) / 60;
    let second = seconds % 60;
    format!("{hour:02}:{minute:02}:{second:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_ui_core::gpu::{GpuScene, UiEvent, UiPainter, UiRect, UiRuntimeState};

    #[test]
    fn node_ui_layout_has_no_structural_issues_on_desktop_size() {
        let report = fixture_report();
        let layout = build_layout(&report, ActivePanel::MachineReport, 1180.0, 760.0, 0.0);

        let issues = layout.layout_issues(UiRect::new(0.0, 0.0, 1180.0, 760.0));

        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn node_ui_layout_has_no_structural_issues_on_minimum_size() {
        let report = fixture_report();
        let layout = build_layout(&report, ActivePanel::MachineReport, 760.0, 520.0, 0.0);

        let issues = layout.layout_issues(UiRect::new(0.0, 0.0, 760.0, 520.0));

        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn node_ui_all_panels_have_structural_layouts() {
        let report = fixture_report();
        for panel in [
            ActivePanel::MachineReport,
            ActivePanel::NodeInstances,
            ActivePanel::Storage,
            ActivePanel::Trust,
        ] {
            let layout = build_layout(&report, panel, 1180.0, 760.0, 0.0);
            let issues = layout.layout_issues(UiRect::new(0.0, 0.0, 1180.0, 760.0));

            assert!(issues.is_empty(), "{panel:?}: {issues:?}");
        }
    }

    #[test]
    fn trust_panel_exposes_control_plane_actions() {
        let report = fixture_report();
        let layout = trust_panel(&report, 932.0);
        let mut scene = GpuScene::new(BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            layout.render(&mut ui, UiRect::new(0.0, 0.0, 932.0, 1200.0));
        }

        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 21_002),
            "trust panel should expose device admission setup action: {:?}",
            scene.hits()
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 21_201),
            "trust panel should expose user master key sealing action"
        );
    }

    #[test]
    fn node_sidebar_nav_items_emit_runtime_actions() {
        let report = fixture_report();
        let layout = build_layout(&report, ActivePanel::MachineReport, 1180.0, 760.0, 0.0);
        let mut scene = GpuScene::new(BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            layout.render(&mut ui, UiRect::new(0.0, 0.0, 1180.0, 760.0));
        }

        let hit = scene
            .hits()
            .iter()
            .copied()
            .find(|hit| hit.kind == HitKind::MenuItem && hit.id == NAV_STORAGE_ID)
            .expect("storage nav item should render as a menu hit");

        let mut runtime = UiRuntimeState::default();
        assert_eq!(
            runtime.handle_event(
                &scene,
                UiEvent::PointerDown {
                    x: hit.x + 4.0,
                    y: hit.y + 4.0,
                },
            ),
            UiAction::Activated(hit),
        );
        assert_eq!(
            nav_panel_for_id(hit.id),
            Some(ActivePanel::Storage),
            "node event handler must be able to route the emitted menu id",
        );
    }

    fn fixture_report() -> MachineReport {
        MachineReport {
            refreshed_at: "12:34:56".to_string(),
            platform: "linux",
            cores: "16".to_string(),
            memory: "64.0 GB".to_string(),
            capability_count: 42,
            device_count: 24,
            sections: vec![
                section(
                    "Compute",
                    "GPU and accelerator devices",
                    ACCENT,
                    vec![
                        "GPU 0000:c1:00.0 (1002:15bf)".to_string(),
                        "Linux NPU accel0".to_string(),
                        "AMD XDNA NPU".to_string(),
                    ],
                ),
                section(
                    "Network",
                    "WiFi, Bluetooth, NFC, and CEC",
                    BLUE,
                    vec![
                        "wlan0 connected".to_string(),
                        "bluetooth hci0".to_string(),
                        "nfc none".to_string(),
                    ],
                ),
                section("Power", "Battery and power supply state", AMBER, Vec::new()),
            ],
        }
    }
}
