//! shadcn-facing event adapter for production UI code.

use std::string::String;

use super::*;

#[derive(Clone, Debug, PartialEq)]
pub enum UiShadcnEvent {
    None,
    OnClick {
        id: u32,
        source: HitKind,
    },
    OnFocus {
        id: Option<u32>,
        source: Option<HitKind>,
    },
    OnHover {
        id: Option<u32>,
        source: Option<HitKind>,
    },
    OnCheckedChange {
        id: u32,
        checked: bool,
    },
    OnValueChange {
        id: u32,
        value: UiShadcnEventValue,
    },
    OnOpenChange {
        id: u32,
        open: bool,
    },
    OnSelect {
        id: u32,
        source: HitKind,
    },
    OnSubmit {
        id: u32,
    },
    OnCancel,
    Raw(UiAction),
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiShadcnEventValue {
    Text(String),
    Number(f32),
    ScrollOffset(f32),
    TabIndex(usize),
}

impl UiShadcnEvent {
    pub fn id(&self) -> Option<u32> {
        match self {
            Self::None | Self::OnCancel | Self::Raw(_) => None,
            Self::OnClick { id, .. }
            | Self::OnCheckedChange { id, .. }
            | Self::OnValueChange { id, .. }
            | Self::OnOpenChange { id, .. }
            | Self::OnSelect { id, .. }
            | Self::OnSubmit { id } => Some(*id),
            Self::OnFocus { id, .. } | Self::OnHover { id, .. } => *id,
        }
    }

    fn or_radio_value(self, id: u32, context: Option<&UiShadcnEventContext>) -> Self {
        let Some(value) = context.and_then(|context| context.radio_value_for_id(id)) else {
            return self;
        };
        Self::OnValueChange {
            id,
            value: UiShadcnEventValue::Text(value.to_string()),
        }
    }
}

pub fn shadcn_event_from_action(action: &UiAction) -> UiShadcnEvent {
    shadcn_event_from_action_with_context(action, None)
}

pub fn shadcn_event_from_action_with_context(
    action: &UiAction,
    context: Option<&UiShadcnEventContext>,
) -> UiShadcnEvent {
    match action {
        UiAction::None => UiShadcnEvent::None,
        UiAction::Hovered(hit) => UiShadcnEvent::OnHover {
            id: hit.map(|hit| hit.id),
            source: hit.map(|hit| hit.kind),
        },
        UiAction::Focused(hit) => UiShadcnEvent::OnFocus {
            id: hit.map(|hit| hit.id),
            source: hit.map(|hit| hit.kind),
        },
        UiAction::Activated(hit) => match hit.kind {
            HitKind::MenuItem
                if context
                    .and_then(|context| context.select_value_for_id(hit.id))
                    .is_some() =>
            {
                UiShadcnEvent::OnValueChange {
                    id: hit.id,
                    value: UiShadcnEventValue::Text(
                        context
                            .and_then(|context| context.select_value_for_id(hit.id))
                            .unwrap_or_default()
                            .to_string(),
                    ),
                }
            }
            HitKind::MenuItem
            | HitKind::ListRow
            | HitKind::Breadcrumb
            | HitKind::TreeItem
            | HitKind::AppLauncherItem
            | HitKind::TransactionRow => UiShadcnEvent::OnSelect {
                id: hit.id,
                source: hit.kind,
            },
            _ => UiShadcnEvent::OnClick {
                id: hit.id,
                source: hit.kind,
            },
        },
        UiAction::Toggled { id, on } => UiShadcnEvent::OnCheckedChange {
            id: *id,
            checked: *on,
        }
        .or_radio_value(*id, context),
        UiAction::TabSelected { id } => UiShadcnEvent::OnValueChange {
            id: *id,
            value: UiShadcnEventValue::TabIndex(
                context
                    .and_then(|context| context.tab_index_for_id(*id))
                    .unwrap_or(0),
            ),
        },
        UiAction::SliderChanged { id, value } => UiShadcnEvent::OnValueChange {
            id: *id,
            value: UiShadcnEventValue::Number(*value),
        },
        UiAction::OpenChanged { id, open } => UiShadcnEvent::OnOpenChange {
            id: *id,
            open: *open,
        },
        UiAction::ScrollChanged { id, offset } => UiShadcnEvent::OnValueChange {
            id: *id,
            value: UiShadcnEventValue::ScrollOffset(*offset),
        },
        UiAction::TextChanged { id, value } => UiShadcnEvent::OnValueChange {
            id: *id,
            value: UiShadcnEventValue::Text(value.clone()),
        },
        UiAction::Submitted { id } => UiShadcnEvent::OnSubmit { id: *id },
        UiAction::Cancelled => UiShadcnEvent::OnCancel,
        UiAction::DragStarted { .. }
        | UiAction::DragMoved { .. }
        | UiAction::Dropped { .. }
        | UiAction::Reordered { .. }
        | UiAction::DragCancelled { .. } => UiShadcnEvent::Raw(action.clone()),
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiShadcnEventContext {
    tab_groups: Vec<(u32, usize)>,
    select_options: Vec<(u32, String)>,
    radio_options: Vec<(u32, String)>,
}

impl UiShadcnEventContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tab_group(mut self, base_id: u32, len: usize) -> Self {
        self.tab_groups.push((base_id, len));
        self
    }

    pub fn add_tab_group(&mut self, base_id: u32, len: usize) {
        self.tab_groups.push((base_id, len));
    }

    pub fn select_options(mut self, base_id: u32, values: &[&str]) -> Self {
        self.add_select_options(base_id, values);
        self
    }

    pub fn add_select_options(&mut self, base_id: u32, values: &[&str]) {
        self.select_options.extend(
            values
                .iter()
                .enumerate()
                .map(|(index, value)| (base_id + index as u32, (*value).to_string())),
        );
    }

    pub fn radio_options(mut self, base_id: u32, values: &[&str]) -> Self {
        self.add_radio_options(base_id, values);
        self
    }

    pub fn add_radio_options(&mut self, base_id: u32, values: &[&str]) {
        self.radio_options.extend(
            values
                .iter()
                .enumerate()
                .map(|(index, value)| (base_id + index as u32, (*value).to_string())),
        );
    }

    fn tab_index_for_id(&self, id: u32) -> Option<usize> {
        self.tab_groups.iter().find_map(|(base_id, len)| {
            (id >= *base_id && id < *base_id + *len as u32).then_some((id - *base_id) as usize)
        })
    }

    fn select_value_for_id(&self, id: u32) -> Option<&str> {
        self.select_options
            .iter()
            .find_map(|(option_id, value)| (*option_id == id).then_some(value.as_str()))
    }

    fn radio_value_for_id(&self, id: u32) -> Option<&str> {
        self.radio_options
            .iter()
            .find_map(|(option_id, value)| (*option_id == id).then_some(value.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapts_raw_runtime_actions_to_shadcn_event_names() {
        assert_eq!(
            shadcn_event_from_action(&UiAction::Activated(GpuHit::new(
                HitKind::Button,
                7,
                0.0,
                0.0,
                10.0,
                10.0
            ))),
            UiShadcnEvent::OnClick {
                id: 7,
                source: HitKind::Button,
            }
        );
        assert_eq!(
            shadcn_event_from_action(&UiAction::Toggled { id: 8, on: true }),
            UiShadcnEvent::OnCheckedChange {
                id: 8,
                checked: true,
            }
        );
        assert_eq!(
            shadcn_event_from_action(&UiAction::OpenChanged { id: 9, open: true }),
            UiShadcnEvent::OnOpenChange { id: 9, open: true }
        );
        assert_eq!(
            shadcn_event_from_action(&UiAction::TextChanged {
                id: 10,
                value: "hello".to_string(),
            }),
            UiShadcnEvent::OnValueChange {
                id: 10,
                value: UiShadcnEventValue::Text("hello".to_string()),
            }
        );
    }

    #[test]
    fn adapts_selection_and_tab_context() {
        assert_eq!(
            shadcn_event_from_action(&UiAction::Activated(GpuHit::new(
                HitKind::MenuItem,
                12,
                0.0,
                0.0,
                10.0,
                10.0
            ))),
            UiShadcnEvent::OnSelect {
                id: 12,
                source: HitKind::MenuItem,
            }
        );

        let context = UiShadcnEventContext::new().tab_group(20, 3);
        assert_eq!(
            shadcn_event_from_action_with_context(
                &UiAction::TabSelected { id: 22 },
                Some(&context)
            ),
            UiShadcnEvent::OnValueChange {
                id: 22,
                value: UiShadcnEventValue::TabIndex(2),
            }
        );

        let context = UiShadcnEventContext::new().select_options(50, &["next", "svelte"]);
        assert_eq!(
            shadcn_event_from_action_with_context(
                &UiAction::Activated(GpuHit::new(HitKind::MenuItem, 51, 0.0, 0.0, 10.0, 10.0)),
                Some(&context)
            ),
            UiShadcnEvent::OnValueChange {
                id: 51,
                value: UiShadcnEventValue::Text("svelte".to_string()),
            }
        );
    }
}
