#[derive(Debug)]
pub struct ThirdPartyModule {
    pub name: &'static str,
    pub default_action: &'static str,
    pub breakage_risk: &'static str,
    pub rationale: &'static str,
}

pub const MODULES: &[ThirdPartyModule] = &[
    ThirdPartyModule {
        name: "Google Play services",
        default_action: "preserve_minimal",
        breakage_risk: "medium",
        rationale: "often carries auth, push, maps, safety, or wearable integrations; analytics should remain opt-out",
    },
];

pub fn configure_defaults() {
    for module in MODULES {
        let _ = module;
    }
}
