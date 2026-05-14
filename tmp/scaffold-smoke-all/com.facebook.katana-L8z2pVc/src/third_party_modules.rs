#[derive(Debug)]
pub struct ThirdPartyModule {
    pub name: &'static str,
    pub default_action: &'static str,
    pub breakage_risk: &'static str,
    pub rationale: &'static str,
}

pub const MODULES: &[ThirdPartyModule] = &[
    ThirdPartyModule {
        name: "Meta/Facebook SDK",
        default_action: "preserve_if_login_or_share_required",
        breakage_risk: "medium",
        rationale: "may provide login, sharing, attribution, or web redirect handling; make optional unless a workflow uses it",
    },
];

pub fn configure_defaults() {
    for module in MODULES {
        let _ = module;
    }
}
