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
        name: "OkHttp",
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
