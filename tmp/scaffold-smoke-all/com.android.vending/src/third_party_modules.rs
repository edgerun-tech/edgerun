#[derive(Debug)]
pub struct ThirdPartyModule {
    pub name: &'static str,
    pub default_action: &'static str,
    pub breakage_risk: &'static str,
    pub rationale: &'static str,
}

pub const MODULES: &[ThirdPartyModule] = &[
    ThirdPartyModule {
        name: "AndroidX Room database",
        default_action: "preserve_if_reachable",
        breakage_risk: "unknown",
        rationale: "module appears in the app; dynamic trace should decide whether it is required",
    },
    ThirdPartyModule {
        name: "AndroidX WorkManager",
        default_action: "preserve_if_reachable",
        breakage_risk: "unknown",
        rationale: "module appears in the app; dynamic trace should decide whether it is required",
    },
    ThirdPartyModule {
        name: "Bouncy Castle crypto",
        default_action: "preserve_if_reachable",
        breakage_risk: "unknown",
        rationale: "module appears in the app; dynamic trace should decide whether it is required",
    },
    ThirdPartyModule {
        name: "Chromium/WebView code",
        default_action: "replace_gradually",
        breakage_risk: "high",
        rationale: "runtime or rendering framework; rebuild screen-by-screen instead of silently dropping it",
    },
    ThirdPartyModule {
        name: "Firebase",
        default_action: "preserve_minimal",
        breakage_risk: "medium",
        rationale: "often carries auth, push, maps, safety, or wearable integrations; analytics should remain opt-out",
    },
    ThirdPartyModule {
        name: "Google Play libraries",
        default_action: "preserve_if_reachable",
        breakage_risk: "unknown",
        rationale: "module appears in the app; dynamic trace should decide whether it is required",
    },
    ThirdPartyModule {
        name: "Google Play services",
        default_action: "preserve_minimal",
        breakage_risk: "medium",
        rationale: "often carries auth, push, maps, safety, or wearable integrations; analytics should remain opt-out",
    },
    ThirdPartyModule {
        name: "Square/OkHttp/Moshi stack",
        default_action: "preserve_if_reachable",
        breakage_risk: "unknown",
        rationale: "module appears in the app; dynamic trace should decide whether it is required",
    },
];

pub fn configure_defaults() {
    for module in MODULES {
        let _ = module;
    }
}
