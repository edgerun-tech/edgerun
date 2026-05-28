//! Rust-side mirror for `edgerun-zig/src/ui_stream.zig`.
//!
//! This module intentionally does not define a new UI protocol. It only emits
//! the exact compact binary patch bytes that the Zig UI stream already uses.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MessageType {
    Tree = 0,
    Patch = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PatchKind {
    TextValue = 0,
    AccordionOpen = 1,
    Alert = 2,
    AlertDialog = 3,
    CalendarSelectedDay = 4,
    CarouselLabel = 5,
    ChartLabel = 6,
    ComboboxSelected = 7,
    CardText = 8,
    EmptyText = 9,
    BadgeLabel = 10,
    AvatarLabel = 11,
    KbdLabel = 12,
    LabelValue = 13,
    BreadcrumbCurrent = 14,
    MenubarActive = 15,
    NavigationMenuActive = 16,
    CommandPlaceholder = 17,
    ContextMenu = 18,
    Dialog = 19,
    DirectionActive = 20,
    Drawer = 21,
    DropdownMenu = 22,
    FieldPlaceholder = 23,
    HoverCardContent = 24,
    InputOtpValue = 25,
    ButtonLabel = 26,
    ButtonGroupActive = 27,
    ToggleGroupActive = 28,
    TogglePressed = 29,
    InputPlaceholder = 30,
    InputGroupPlaceholder = 31,
    TextareaPlaceholder = 32,
    SelectLabel = 33,
    CheckboxChecked = 34,
    RadioSelected = 35,
    SwitchChecked = 36,
    PaginationPage = 37,
    PopoverContent = 38,
    ResizableRatio = 39,
    Sheet = 40,
    SidebarItem = 41,
    ProgressValue = 42,
    SliderValue = 43,
    TabsActive = 44,
    TableRow = 45,
    TooltipContent = 46,
    Toast = 47,
    RowItem = 48,
    RectColor = 49,
    StyleColor = 50,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodeError {
    StringTooLong,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AgentComponent {
    AssistantDraft = 1,
    Status = 2,
    ToolCall = 3,
    Stdout = 4,
    Stderr = 5,
    DiffPreview = 6,
    Input = 7,
    RunButton = 8,
}

impl AgentComponent {
    pub const fn id(self) -> u8 {
        self as u8
    }
}

pub fn encode_patch(kind: PatchKind, component_id: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(2 + payload.len());
    out.push(kind as u8);
    out.push(component_id);
    out.extend_from_slice(payload);
    out
}

pub fn encode_patch_message(kind: PatchKind, component_id: u8, payload: &[u8]) -> Vec<u8> {
    let patch = encode_patch(kind, component_id, payload);
    let mut out = Vec::with_capacity(1 + patch.len());
    out.push(MessageType::Patch as u8);
    out.extend_from_slice(&patch);
    out
}

pub fn encode_bool(kind: PatchKind, component_id: u8, value: bool) -> Vec<u8> {
    encode_patch(kind, component_id, &[u8::from(value)])
}

pub fn encode_u16(kind: PatchKind, component_id: u8, value: u16) -> Vec<u8> {
    encode_patch(kind, component_id, &value.to_le_bytes())
}

pub fn encode_f32(kind: PatchKind, component_id: u8, value: f32) -> Vec<u8> {
    encode_patch(kind, component_id, &value.to_bits().to_le_bytes())
}

pub fn encode_string(kind: PatchKind, component_id: u8, value: &str) -> Result<Vec<u8>, EncodeError> {
    let len = u8::try_from(value.len()).map_err(|_| EncodeError::StringTooLong)?;
    let mut payload = Vec::with_capacity(1 + value.len());
    payload.push(len);
    payload.extend_from_slice(value.as_bytes());
    Ok(encode_patch(kind, component_id, &payload))
}

pub fn encode_two_strings(kind: PatchKind, component_id: u8, first: &str, second: &str) -> Result<Vec<u8>, EncodeError> {
    let first_len = u8::try_from(first.len()).map_err(|_| EncodeError::StringTooLong)?;
    let second_len = u8::try_from(second.len()).map_err(|_| EncodeError::StringTooLong)?;
    let mut payload = Vec::with_capacity(2 + first.len() + second.len());
    payload.push(first_len);
    payload.extend_from_slice(first.as_bytes());
    payload.push(second_len);
    payload.extend_from_slice(second.as_bytes());
    Ok(encode_patch(kind, component_id, &payload))
}

pub fn encode_two_strings_bool(kind: PatchKind, component_id: u8, first: &str, second: &str, flag: bool) -> Result<Vec<u8>, EncodeError> {
    let first_len = u8::try_from(first.len()).map_err(|_| EncodeError::StringTooLong)?;
    let second_len = u8::try_from(second.len()).map_err(|_| EncodeError::StringTooLong)?;
    let mut payload = Vec::with_capacity(3 + first.len() + second.len());
    payload.push(first_len);
    payload.extend_from_slice(first.as_bytes());
    payload.push(second_len);
    payload.extend_from_slice(second.as_bytes());
    payload.push(u8::from(flag));
    Ok(encode_patch(kind, component_id, &payload))
}

pub fn encode_color(kind: PatchKind, component_id: u8, color: Color) -> Vec<u8> {
    encode_patch(kind, component_id, &[color.r, color.g, color.b, color.a])
}

pub fn truncate_utf8(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

pub mod agent {
    use super::*;

    pub fn status(text: &str) -> Vec<u8> {
        encode_string(
            PatchKind::LabelValue,
            AgentComponent::Status.id(),
            truncate_utf8(text, 255),
        )
        .expect("status text is truncated to ui_stream string limit")
    }

    pub fn progress(value: f32) -> Vec<u8> {
        encode_f32(PatchKind::ProgressValue, AgentComponent::Status.id(), value)
    }

    pub fn assistant_draft(text: &str) -> Vec<u8> {
        encode_two_strings(
            PatchKind::CardText,
            AgentComponent::AssistantDraft.id(),
            "assistant",
            truncate_utf8(text, 255),
        )
        .expect("assistant draft text is truncated to ui_stream string limit")
    }

    pub fn tool_call(name: &str, detail: &str) -> Vec<u8> {
        encode_two_strings(
            PatchKind::RowItem,
            AgentComponent::ToolCall.id(),
            truncate_utf8(name, 255),
            truncate_utf8(detail, 255),
        )
        .expect("tool call text is truncated to ui_stream string limit")
    }

    pub fn stdout(text: &str) -> Vec<u8> {
        encode_two_strings(
            PatchKind::RowItem,
            AgentComponent::Stdout.id(),
            "stdout",
            truncate_utf8(text, 255),
        )
        .expect("stdout text is truncated to ui_stream string limit")
    }

    pub fn stderr(text: &str) -> Vec<u8> {
        encode_two_strings(
            PatchKind::RowItem,
            AgentComponent::Stderr.id(),
            "stderr",
            truncate_utf8(text, 255),
        )
        .expect("stderr text is truncated to ui_stream string limit")
    }

    pub fn diff_preview(path: &str, summary: &str) -> Vec<u8> {
        encode_two_strings(
            PatchKind::RowItem,
            AgentComponent::DiffPreview.id(),
            truncate_utf8(path, 255),
            truncate_utf8(summary, 255),
        )
        .expect("diff preview text is truncated to ui_stream string limit")
    }

    pub fn run_button(label: &str) -> Vec<u8> {
        encode_string(
            PatchKind::ButtonLabel,
            AgentComponent::RunButton.id(),
            truncate_utf8(label, 255),
        )
        .expect("button label is truncated to ui_stream string limit")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_patch_matches_zig_layout() {
        assert_eq!(
            encode_string(PatchKind::ButtonLabel, 8, "run").unwrap(),
            vec![26, 8, 3, b'r', b'u', b'n']
        );
    }

    #[test]
    fn two_string_patch_matches_zig_layout() {
        assert_eq!(
            encode_two_strings(PatchKind::RowItem, 3, "tool", "running").unwrap(),
            vec![48, 3, 4, b't', b'o', b'o', b'l', 7, b'r', b'u', b'n', b'n', b'i', b'n', b'g']
        );
    }

    #[test]
    fn f32_patch_matches_zig_layout() {
        assert_eq!(
            encode_f32(PatchKind::ProgressValue, 2, 0.5),
            vec![42, 2, 0, 0, 0, 63]
        );
    }

    #[test]
    fn color_patch_matches_zig_layout() {
        assert_eq!(
            encode_color(PatchKind::StyleColor, 1, Color { r: 1, g: 2, b: 3, a: 4 }),
            vec![50, 1, 1, 2, 3, 4]
        );
    }

    #[test]
    fn patch_message_keeps_message_type_outside_patch_payload() {
        assert_eq!(
            encode_patch_message(PatchKind::LabelValue, 2, &[2, b'o', b'k']),
            vec![1, 13, 2, 2, b'o', b'k']
        );
    }
}
