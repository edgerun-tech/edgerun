use crate::approvals::NetworkApprovalProtocol;
use crate::compat::network_proxy::NetworkDecisionSource;
use crate::compat::network_proxy::NetworkPolicyDecision;

#[derive(Debug, Clone, PartialEq, Eq, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct NetworkPolicyDecisionPayload {
    pub decision: NetworkPolicyDecision,
    pub source: NetworkDecisionSource,
    #[schemars(default)]
    pub protocol: Option<NetworkApprovalProtocol>,
    pub host: Option<String>,
    pub reason: Option<String>,
    pub port: Option<u16>,
}

impl NetworkPolicyDecisionPayload {
    pub fn is_ask_from_decider(&self) -> bool {
        self.decision == NetworkPolicyDecision::Ask && self.source == NetworkDecisionSource::Decider
    }
}
