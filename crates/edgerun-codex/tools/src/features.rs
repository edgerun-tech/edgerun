use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Feature {
    Apps,
    ApplyPatchFreeform,
    CodeMode,
    CodeModeOnly,
    Collab,
    DefaultModeRequestUserInput,
    ExecPermissionApprovals,
    Goals,
    ImageGeneration,
    MultiAgentV2,
    Plugins,
    RequestPermissionsTool,
    ShellTool,
    ShellZshFork,
    SpawnCsv,
    ToolSearch,
    ToolSuggest,
    UnifiedExec,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Features {
    enabled: BTreeSet<Feature>,
}

impl Features {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn with_defaults() -> Self {
        let mut features = Self::default();
        features.enable(Feature::ApplyPatchFreeform);
        features.enable(Feature::ShellTool);
        features
    }

    pub fn enable(&mut self, feature: Feature) {
        self.enabled.insert(feature);
    }

    pub fn disable(&mut self, feature: Feature) {
        self.enabled.remove(&feature);
    }

    pub fn enabled(&self, feature: Feature) -> bool {
        self.enabled.contains(&feature)
    }
}
