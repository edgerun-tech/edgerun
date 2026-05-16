use std::env;

use edgerun_ui_core::gpu::sdl::{SdlGlWindowOptions, instantiate_sdl_app};
use edgerun_ui_core::gpu::{
    Color4, UiAction, UiAppControl, UiNode, UiRect, UiShadcnDemoGalleryState, UiSurfaceApp,
    build_shadcn_component_preview_by_identifier, build_shadcn_demo_gallery_with_state,
    scroll_area,
};

struct ShadcnPreviewApp {
    identifier: Option<String>,
    shadcn: UiShadcnDemoGalleryState,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("ui-preview-sdl-shadcn: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let identifier = env::args().nth(1);
    let title = match identifier.as_deref() {
        Some("shadcn-demos") | None => "EdgeRun shadcn Demo Gallery",
        Some(_) => "EdgeRun shadcn Component Preview",
    };
    instantiate_sdl_app(
        SdlGlWindowOptions::new(title, 1440, 940, Color4::rgb_u8(248, 250, 252)).min_size(980, 720),
        ShadcnPreviewApp {
            identifier,
            shadcn: UiShadcnDemoGalleryState::default(),
        },
    )
}

impl UiSurfaceApp for ShadcnPreviewApp {
    fn surface(&mut self, _viewport: UiRect) -> UiNode {
        if matches!(self.identifier.as_deref(), Some("shadcn-demos") | None) {
            return build_shadcn_demo_gallery_with_state(&self.shadcn);
        }

        let body = match self.identifier.as_deref() {
            Some(identifier) => build_shadcn_component_preview_by_identifier(identifier)
                .unwrap_or_else(|| {
                    panic!("unknown shadcn component preview identifier: {identifier}")
                }),
            None => unreachable!("handled above"),
        };

        scroll_area("bg-bg p-6 gap-4 h-full", 0.0)
            .scroll_id(42_300)
            .child(body)
    }

    fn handle_action(&mut self, action: UiAction) -> UiAppControl {
        if self.shadcn.apply_action(&action) {
            UiAppControl::dirty()
        } else if matches!(
            action,
            UiAction::Hovered(_) | UiAction::Focused(_) | UiAction::Activated(_)
        ) {
            UiAppControl::dirty()
        } else {
            UiAppControl::clean()
        }
    }

    fn tick(&mut self, _delta_ms: u32) -> UiAppControl {
        UiAppControl::dirty()
    }
}
