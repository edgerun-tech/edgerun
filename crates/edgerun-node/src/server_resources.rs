//! Typed server resource state for event-driven node hosting.
//!
//! This is not a generic configuration layer. It models resources that are
//! changed only by signed stream commands, then derives the runtime plan.

use edgerun_core::collections::{HashMap, HashSet};
use edgerun_proto::edgerun::v0::common::ObjectRef;
use edgerun_storage::NodeStore;
use prost::Message;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompiledBootstrapPolicy {
    pub node_label: &'static str,
    pub stream_id: [u8; 32],
    pub controller_id: [u8; 64],
    pub bootstrap_relays: &'static [&'static str],
}

pub mod command_type {
    pub const CLAIM_DOMAIN: i32 = 1012;
    pub const RELEASE_DOMAIN: i32 = 1013;
    pub const ADD_MAILBOX: i32 = 1014;
    pub const REMOVE_MAILBOX: i32 = 1015;
    pub const ADD_ALIAS: i32 = 1016;
    pub const REMOVE_ALIAS: i32 = 1017;
    pub const AUTHORIZE_CONTENT_SOURCE: i32 = 1018;
    pub const PUBLISH_WEBSITE: i32 = 1019;
    pub const UNPUBLISH_WEBSITE: i32 = 1020;
    pub const SET_AUTHORITATIVE_DNS: i32 = 1021;
    pub const REQUEST_CERTIFICATE: i32 = 1022;
    pub const SET_SERVICE_POLICY: i32 = 1023;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerResourceEventKind {
    ClaimDomain = 1,
    ReleaseDomain = 2,
    AddMailbox = 3,
    RemoveMailbox = 4,
    AddAlias = 5,
    RemoveAlias = 6,
    AuthorizeContentSource = 7,
    PublishWebsite = 8,
    UnpublishWebsite = 9,
    SetAuthoritativeDns = 10,
    RequestCertificate = 11,
    SetServicePolicy = 12,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentRef {
    pub repo: String,
    pub commit: String,
    pub path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentSourceAuth {
    pub repo: String,
    pub allowed_ref: String,
    pub allowed_paths: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WebsiteBinding {
    pub domain: String,
    pub content_ref: ContentRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerResourceEvent {
    ClaimDomain { domain: String },
    ReleaseDomain { domain: String },
    AddMailbox { address: String },
    RemoveMailbox { address: String },
    AddAlias { address: String, target: String },
    RemoveAlias { address: String },
    AuthorizeContentSource { repo: String, allowed_ref: String, allowed_paths: Vec<String> },
    PublishWebsite { domain: String, content_ref: ContentRef },
    UnpublishWebsite { domain: String },
    SetAuthoritativeDns { domain: String, enabled: bool },
    RequestCertificate { name: String },
    SetServicePolicy { service: String, policy: String },
}

impl ServerResourceEvent {
    pub fn kind(&self) -> ServerResourceEventKind {
        match self {
            Self::ClaimDomain { .. } => ServerResourceEventKind::ClaimDomain,
            Self::ReleaseDomain { .. } => ServerResourceEventKind::ReleaseDomain,
            Self::AddMailbox { .. } => ServerResourceEventKind::AddMailbox,
            Self::RemoveMailbox { .. } => ServerResourceEventKind::RemoveMailbox,
            Self::AddAlias { .. } => ServerResourceEventKind::AddAlias,
            Self::RemoveAlias { .. } => ServerResourceEventKind::RemoveAlias,
            Self::AuthorizeContentSource { .. } => ServerResourceEventKind::AuthorizeContentSource,
            Self::PublishWebsite { .. } => ServerResourceEventKind::PublishWebsite,
            Self::UnpublishWebsite { .. } => ServerResourceEventKind::UnpublishWebsite,
            Self::SetAuthoritativeDns { .. } => ServerResourceEventKind::SetAuthoritativeDns,
            Self::RequestCertificate { .. } => ServerResourceEventKind::RequestCertificate,
            Self::SetServicePolicy { .. } => ServerResourceEventKind::SetServicePolicy,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ServerResourceProjection {
    pub domains: HashSet<String>,
    pub mailboxes: HashSet<String>,
    pub aliases: HashMap<String, String>,
    pub content_sources: HashMap<String, ContentSourceAuth>,
    pub websites: HashMap<String, WebsiteBinding>,
    pub certificates: HashSet<String>,
    pub authoritative_dns: HashSet<String>,
    pub service_policies: HashMap<String, String>,
    pub derived_dns_records: Vec<DerivedDnsRecord>,
    pub required_ports: HashSet<u16>,
    pub required_services: HashSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivedDnsRecord {
    pub name: String,
    pub rr_type: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NodeFacts {
    pub public_ipv4: Option<String>,
    pub public_ipv6: Option<String>,
    pub mail_host_prefix: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedServerPlan {
    pub required_services: HashSet<String>,
    pub required_ports: HashSet<u16>,
    pub dns_records: Vec<DerivedDnsRecord>,
    pub certificates: HashSet<String>,
    pub mail_routes: Vec<MailRoute>,
    pub http_routes: Vec<HttpRoute>,
    pub service_policies: HashMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MailRoute {
    pub address: String,
    pub target: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpRoute {
    pub domain: String,
    pub content_ref: ContentRef,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationEventKind {
    Planned = 1,
    Staged = 2,
    HealthPassed = 3,
    Promoted = 4,
    Rollback = 5,
    Failed = 6,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationEvent {
    pub generation_id: [u8; 32],
    pub kind: GenerationEventKind,
    pub plan_hash: [u8; 32],
    pub reason: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ClaimDomainPayload {
    #[prost(uint32, tag = "1")]
    pub payload_version: u32,
    #[prost(string, tag = "2")]
    pub domain: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ReleaseDomainPayload {
    #[prost(uint32, tag = "1")]
    pub payload_version: u32,
    #[prost(string, tag = "2")]
    pub domain: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct AddMailboxPayload {
    #[prost(uint32, tag = "1")]
    pub payload_version: u32,
    #[prost(string, tag = "2")]
    pub address: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct RemoveMailboxPayload {
    #[prost(uint32, tag = "1")]
    pub payload_version: u32,
    #[prost(string, tag = "2")]
    pub address: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct AddAliasPayload {
    #[prost(uint32, tag = "1")]
    pub payload_version: u32,
    #[prost(string, tag = "2")]
    pub address: String,
    #[prost(string, tag = "3")]
    pub target: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct RemoveAliasPayload {
    #[prost(uint32, tag = "1")]
    pub payload_version: u32,
    #[prost(string, tag = "2")]
    pub address: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct AuthorizeContentSourcePayload {
    #[prost(uint32, tag = "1")]
    pub payload_version: u32,
    #[prost(string, tag = "2")]
    pub repo: String,
    #[prost(string, tag = "3")]
    pub allowed_ref: String,
    #[prost(string, repeated, tag = "4")]
    pub allowed_paths: Vec<String>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ContentRefPayload {
    #[prost(string, tag = "1")]
    pub repo: String,
    #[prost(string, tag = "2")]
    pub commit: String,
    #[prost(string, tag = "3")]
    pub path: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct PublishWebsitePayload {
    #[prost(uint32, tag = "1")]
    pub payload_version: u32,
    #[prost(string, tag = "2")]
    pub domain: String,
    #[prost(message, optional, tag = "3")]
    pub content_ref: Option<ContentRefPayload>,
}

#[derive(Clone, PartialEq, Message)]
pub struct UnpublishWebsitePayload {
    #[prost(uint32, tag = "1")]
    pub payload_version: u32,
    #[prost(string, tag = "2")]
    pub domain: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct SetAuthoritativeDnsPayload {
    #[prost(uint32, tag = "1")]
    pub payload_version: u32,
    #[prost(string, tag = "2")]
    pub domain: String,
    #[prost(bool, tag = "3")]
    pub enabled: bool,
}

#[derive(Clone, PartialEq, Message)]
pub struct RequestCertificatePayload {
    #[prost(uint32, tag = "1")]
    pub payload_version: u32,
    #[prost(string, tag = "2")]
    pub name: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct SetServicePolicyPayload {
    #[prost(uint32, tag = "1")]
    pub payload_version: u32,
    #[prost(string, tag = "2")]
    pub service: String,
    #[prost(string, tag = "3")]
    pub policy: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ServerResourceCommittedPayload {
    #[prost(uint32, tag = "1")]
    pub payload_version: u32,
    #[prost(int32, tag = "2")]
    pub event_kind: i32,
    #[prost(bytes = "vec", tag = "3")]
    pub event_payload: Vec<u8>,
    #[prost(message, optional, tag = "4")]
    pub origin_command: Option<edgerun_proto::edgerun::v0::common::CommandRef>,
}

pub fn is_server_resource_command(command_type: i32) -> bool {
    matches!(command_type, 1012..=1023)
}

pub fn decode_command_payload(command_type: i32, payload: &[u8]) -> Result<ServerResourceEvent, String> {
    match command_type {
        command_type::CLAIM_DOMAIN => {
            let p = ClaimDomainPayload::decode(payload).map_err(|e| e.to_string())?;
            validate_domain(&p.domain)?;
            Ok(ServerResourceEvent::ClaimDomain { domain: normalize_domain(&p.domain) })
        }
        command_type::RELEASE_DOMAIN => {
            let p = ReleaseDomainPayload::decode(payload).map_err(|e| e.to_string())?;
            validate_domain(&p.domain)?;
            Ok(ServerResourceEvent::ReleaseDomain { domain: normalize_domain(&p.domain) })
        }
        command_type::ADD_MAILBOX => {
            let p = AddMailboxPayload::decode(payload).map_err(|e| e.to_string())?;
            validate_address(&p.address)?;
            Ok(ServerResourceEvent::AddMailbox { address: normalize_address(&p.address) })
        }
        command_type::REMOVE_MAILBOX => {
            let p = RemoveMailboxPayload::decode(payload).map_err(|e| e.to_string())?;
            validate_address(&p.address)?;
            Ok(ServerResourceEvent::RemoveMailbox { address: normalize_address(&p.address) })
        }
        command_type::ADD_ALIAS => {
            let p = AddAliasPayload::decode(payload).map_err(|e| e.to_string())?;
            validate_address(&p.address)?;
            validate_address(&p.target)?;
            Ok(ServerResourceEvent::AddAlias { address: normalize_address(&p.address), target: normalize_address(&p.target) })
        }
        command_type::REMOVE_ALIAS => {
            let p = RemoveAliasPayload::decode(payload).map_err(|e| e.to_string())?;
            validate_address(&p.address)?;
            Ok(ServerResourceEvent::RemoveAlias { address: normalize_address(&p.address) })
        }
        command_type::AUTHORIZE_CONTENT_SOURCE => {
            let p = AuthorizeContentSourcePayload::decode(payload).map_err(|e| e.to_string())?;
            if p.repo.trim().is_empty() { return Err("repo_required".into()); }
            if p.allowed_ref.trim().is_empty() { return Err("allowed_ref_required".into()); }
            if p.allowed_paths.is_empty() { return Err("allowed_paths_required".into()); }
            Ok(ServerResourceEvent::AuthorizeContentSource { repo: p.repo, allowed_ref: p.allowed_ref, allowed_paths: p.allowed_paths })
        }
        command_type::PUBLISH_WEBSITE => {
            let p = PublishWebsitePayload::decode(payload).map_err(|e| e.to_string())?;
            validate_domain(&p.domain)?;
            let c = p.content_ref.ok_or_else(|| "content_ref_required".to_string())?;
            let content_ref = ContentRef { repo: c.repo, commit: c.commit, path: c.path };
            validate_content_ref(&content_ref)?;
            Ok(ServerResourceEvent::PublishWebsite { domain: normalize_domain(&p.domain), content_ref })
        }
        command_type::UNPUBLISH_WEBSITE => {
            let p = UnpublishWebsitePayload::decode(payload).map_err(|e| e.to_string())?;
            validate_domain(&p.domain)?;
            Ok(ServerResourceEvent::UnpublishWebsite { domain: normalize_domain(&p.domain) })
        }
        command_type::SET_AUTHORITATIVE_DNS => {
            let p = SetAuthoritativeDnsPayload::decode(payload).map_err(|e| e.to_string())?;
            validate_domain(&p.domain)?;
            Ok(ServerResourceEvent::SetAuthoritativeDns { domain: normalize_domain(&p.domain), enabled: p.enabled })
        }
        command_type::REQUEST_CERTIFICATE => {
            let p = RequestCertificatePayload::decode(payload).map_err(|e| e.to_string())?;
            validate_domain(&p.name)?;
            Ok(ServerResourceEvent::RequestCertificate { name: normalize_domain(&p.name) })
        }
        command_type::SET_SERVICE_POLICY => {
            let p = SetServicePolicyPayload::decode(payload).map_err(|e| e.to_string())?;
            if p.service.trim().is_empty() { return Err("service_required".into()); }
            if p.policy.trim().is_empty() { return Err("policy_required".into()); }
            Ok(ServerResourceEvent::SetServicePolicy { service: p.service, policy: p.policy })
        }
        _ => Err("not_server_resource_command".into()),
    }
}

pub fn apply_server_resource_event(projection: &mut ServerResourceProjection, event: ServerResourceEvent) -> Result<(), String> {
    match event {
        ServerResourceEvent::ClaimDomain { domain } => { projection.domains.insert(domain); }
        ServerResourceEvent::ReleaseDomain { domain } => {
            projection.domains.remove(&domain);
            projection.authoritative_dns.remove(&domain);
            projection.websites.remove(&domain);
            projection.certificates.remove(&domain);
            projection.mailboxes.retain(|addr| !addr.ends_with(&format!("@{domain}")));
            projection.aliases.retain(|addr, _| !addr.ends_with(&format!("@{domain}")));
        }
        ServerResourceEvent::AddMailbox { address } => {
            let domain = address_domain(&address)?;
            if !projection.domains.contains(domain) { return Err("domain_not_claimed".into()); }
            projection.mailboxes.insert(address);
        }
        ServerResourceEvent::RemoveMailbox { address } => { projection.mailboxes.remove(&address); }
        ServerResourceEvent::AddAlias { address, target } => {
            let domain = address_domain(&address)?;
            if !projection.domains.contains(domain) { return Err("domain_not_claimed".into()); }
            projection.aliases.insert(address, target);
        }
        ServerResourceEvent::RemoveAlias { address } => { projection.aliases.remove(&address); }
        ServerResourceEvent::AuthorizeContentSource { repo, allowed_ref, allowed_paths } => {
            projection.content_sources.insert(repo.clone(), ContentSourceAuth { repo, allowed_ref, allowed_paths });
        }
        ServerResourceEvent::PublishWebsite { domain, content_ref } => {
            if !projection.domains.contains(&domain) { return Err("domain_not_claimed".into()); }
            validate_content_authorized(projection, &content_ref)?;
            projection.websites.insert(domain.clone(), WebsiteBinding { domain, content_ref });
        }
        ServerResourceEvent::UnpublishWebsite { domain } => { projection.websites.remove(&domain); }
        ServerResourceEvent::SetAuthoritativeDns { domain, enabled } => {
            if !projection.domains.contains(&domain) { return Err("domain_not_claimed".into()); }
            if enabled { projection.authoritative_dns.insert(domain); } else { projection.authoritative_dns.remove(&domain); }
        }
        ServerResourceEvent::RequestCertificate { name } => { projection.certificates.insert(name); }
        ServerResourceEvent::SetServicePolicy { service, policy } => { projection.service_policies.insert(service, policy); }
    }
    Ok(())
}

pub fn project_server_resources_from_events<I>(events: I) -> Result<ServerResourceProjection, String>
where
    I: IntoIterator<Item = ServerResourceEvent>,
{
    let mut projection = ServerResourceProjection::default();
    for event in events {
        apply_server_resource_event(&mut projection, event)?;
    }
    let plan = compile_server_plan(&projection, &NodeFacts::default());
    projection.derived_dns_records = plan.dns_records;
    projection.required_ports = plan.required_ports;
    projection.required_services = plan.required_services;
    Ok(projection)
}

pub fn project_server_resources(store: &NodeStore, stream_id: &[u8]) -> Result<ServerResourceProjection, String> {
    let head_seq = match store.get_head(stream_id) {
        Ok(Some((seq, _))) => seq,
        Ok(None) => return Ok(ServerResourceProjection::default()),
        Err(e) => return Err(format!("failed_to_get_head: {e}")),
    };
    if head_seq == 0 {
        return Ok(ServerResourceProjection::default());
    }
    let stream_id_hex = edgerun_core::util::bytes_to_hex(stream_id);
    let mut projection = ServerResourceProjection::default();
    let events = store.list_event_range(&stream_id_hex, 1, head_seq).map_err(|e| format!("failed_to_list_events: {e}"))?;
    for (seq, _hash, _ver) in events {
        let Some((_event, Some(payload_bytes))) = store.get_event_with_payload(stream_id, seq as u64).map_err(|e| format!("failed_to_load_event_{seq}: {e}"))? else { continue; };
        let Ok(result) = edgerun_proto::edgerun::v0::stream::CommandResultPayload::decode(&payload_bytes[..]) else { continue; };
        let Some(result_object) = result.result_object.as_ref() else { continue; };
        let Some(object) = store.get_object(result_object).map_err(|e| format!("failed_to_load_result_object_{seq}: {e}"))? else { continue; };
        if let Ok(resource_event) = decode_committed_resource_event(&object.content) {
            apply_server_resource_event(&mut projection, resource_event)?;
        }
    }
    let plan = compile_server_plan(&projection, &NodeFacts::default());
    projection.derived_dns_records = plan.dns_records;
    projection.required_ports = plan.required_ports;
    projection.required_services = plan.required_services;
    Ok(projection)
}

pub fn compile_server_plan(projection: &ServerResourceProjection, node_facts: &NodeFacts) -> DerivedServerPlan {
    let mut plan = DerivedServerPlan::default();
    plan.service_policies = projection.service_policies.clone();
    for mailbox in &projection.mailboxes {
        require_email(&mut plan);
        plan.mail_routes.push(MailRoute { address: mailbox.clone(), target: mailbox.clone() });
        if let Ok(domain) = address_domain(mailbox) { add_mail_dns(&mut plan, domain, node_facts); }
    }
    for (alias, target) in &projection.aliases {
        require_email(&mut plan);
        plan.mail_routes.push(MailRoute { address: alias.clone(), target: target.clone() });
        if let Ok(domain) = address_domain(alias) { add_mail_dns(&mut plan, domain, node_facts); }
    }
    for website in projection.websites.values() {
        plan.required_services.insert("edgerun-http".into());
        plan.required_ports.extend([80, 443]);
        plan.certificates.insert(website.domain.clone());
        plan.http_routes.push(HttpRoute { domain: website.domain.clone(), content_ref: website.content_ref.clone() });
        if projection.authoritative_dns.contains(&website.domain) {
            if let Some(ipv4) = &node_facts.public_ipv4 { plan.dns_records.push(DerivedDnsRecord { name: website.domain.clone(), rr_type: "A".into(), value: ipv4.clone() }); }
            if let Some(ipv6) = &node_facts.public_ipv6 { plan.dns_records.push(DerivedDnsRecord { name: website.domain.clone(), rr_type: "AAAA".into(), value: ipv6.clone() }); }
        }
    }
    for cert in &projection.certificates { plan.certificates.insert(cert.clone()); }
    plan
}

pub fn encode_committed_resource_event(event: &ServerResourceEvent, origin_command: Option<edgerun_proto::edgerun::v0::common::CommandRef>) -> ServerResourceCommittedPayload {
    let event_payload = match event {
        ServerResourceEvent::ClaimDomain { domain } => ClaimDomainPayload { payload_version: 1, domain: domain.clone() }.encode_to_vec(),
        ServerResourceEvent::ReleaseDomain { domain } => ReleaseDomainPayload { payload_version: 1, domain: domain.clone() }.encode_to_vec(),
        ServerResourceEvent::AddMailbox { address } => AddMailboxPayload { payload_version: 1, address: address.clone() }.encode_to_vec(),
        ServerResourceEvent::RemoveMailbox { address } => RemoveMailboxPayload { payload_version: 1, address: address.clone() }.encode_to_vec(),
        ServerResourceEvent::AddAlias { address, target } => AddAliasPayload { payload_version: 1, address: address.clone(), target: target.clone() }.encode_to_vec(),
        ServerResourceEvent::RemoveAlias { address } => RemoveAliasPayload { payload_version: 1, address: address.clone() }.encode_to_vec(),
        ServerResourceEvent::AuthorizeContentSource { repo, allowed_ref, allowed_paths } => AuthorizeContentSourcePayload { payload_version: 1, repo: repo.clone(), allowed_ref: allowed_ref.clone(), allowed_paths: allowed_paths.clone() }.encode_to_vec(),
        ServerResourceEvent::PublishWebsite { domain, content_ref } => PublishWebsitePayload { payload_version: 1, domain: domain.clone(), content_ref: Some(ContentRefPayload { repo: content_ref.repo.clone(), commit: content_ref.commit.clone(), path: content_ref.path.clone() }) }.encode_to_vec(),
        ServerResourceEvent::UnpublishWebsite { domain } => UnpublishWebsitePayload { payload_version: 1, domain: domain.clone() }.encode_to_vec(),
        ServerResourceEvent::SetAuthoritativeDns { domain, enabled } => SetAuthoritativeDnsPayload { payload_version: 1, domain: domain.clone(), enabled: *enabled }.encode_to_vec(),
        ServerResourceEvent::RequestCertificate { name } => RequestCertificatePayload { payload_version: 1, name: name.clone() }.encode_to_vec(),
        ServerResourceEvent::SetServicePolicy { service, policy } => SetServicePolicyPayload { payload_version: 1, service: service.clone(), policy: policy.clone() }.encode_to_vec(),
    };
    ServerResourceCommittedPayload { payload_version: 1, event_kind: event.kind() as i32, event_payload, origin_command }
}

pub fn decode_committed_resource_event(payload: &[u8]) -> Result<ServerResourceEvent, String> {
    let committed = ServerResourceCommittedPayload::decode(payload).map_err(|e| e.to_string())?;
    let command_type = match committed.event_kind {
        1 => command_type::CLAIM_DOMAIN,
        2 => command_type::RELEASE_DOMAIN,
        3 => command_type::ADD_MAILBOX,
        4 => command_type::REMOVE_MAILBOX,
        5 => command_type::ADD_ALIAS,
        6 => command_type::REMOVE_ALIAS,
        7 => command_type::AUTHORIZE_CONTENT_SOURCE,
        8 => command_type::PUBLISH_WEBSITE,
        9 => command_type::UNPUBLISH_WEBSITE,
        10 => command_type::SET_AUTHORITATIVE_DNS,
        11 => command_type::REQUEST_CERTIFICATE,
        12 => command_type::SET_SERVICE_POLICY,
        _ => return Err("unknown_server_resource_event_kind".into()),
    };
    decode_command_payload(command_type, &committed.event_payload)
}

pub fn result_object_for_event(store: &mut NodeStore, stream_id: &[u8], event: &ServerResourceEvent) -> Result<ObjectRef, String> {
    let payload = encode_committed_resource_event(event, None).encode_to_vec();
    store.put_object(&payload, edgerun_proto::edgerun::v0::common::ObjectKind::DerivedView as i32, &[stream_id.to_vec()]).map_err(|e| format!("storage_failed: {e}"))
}

fn require_email(plan: &mut DerivedServerPlan) {
    plan.required_services.insert("edgerun-email".into());
    plan.required_ports.extend([25, 465, 587, 993]);
}

fn add_mail_dns(plan: &mut DerivedServerPlan, domain: &str, node_facts: &NodeFacts) {
    let mail_host = if node_facts.mail_host_prefix.is_empty() { format!("mail.{domain}") } else { format!("{}.{domain}", node_facts.mail_host_prefix) };
    plan.certificates.insert(mail_host.clone());
    plan.dns_records.push(DerivedDnsRecord { name: domain.into(), rr_type: "MX".into(), value: format!("10 {mail_host}") });
    plan.dns_records.push(DerivedDnsRecord { name: domain.into(), rr_type: "TXT".into(), value: "v=spf1 mx -all".into() });
    plan.dns_records.push(DerivedDnsRecord { name: format!("_dmarc.{domain}"), rr_type: "TXT".into(), value: "v=DMARC1; p=quarantine".into() });
    plan.dns_records.push(DerivedDnsRecord { name: format!("_mta-sts.{domain}"), rr_type: "TXT".into(), value: "v=STSv1; id=edgerun".into() });
    plan.dns_records.push(DerivedDnsRecord { name: format!("_smtp._tls.{domain}"), rr_type: "TXT".into(), value: "v=TLSRPTv1".into() });
    plan.dns_records.push(DerivedDnsRecord { name: format!("default._domainkey.{domain}"), rr_type: "TXT".into(), value: "v=DKIM1; k=ed25519; p=<derived>".into() });
    if let Some(ipv4) = &node_facts.public_ipv4 { plan.dns_records.push(DerivedDnsRecord { name: mail_host.clone(), rr_type: "A".into(), value: ipv4.clone() }); }
    if let Some(ipv6) = &node_facts.public_ipv6 { plan.dns_records.push(DerivedDnsRecord { name: mail_host, rr_type: "AAAA".into(), value: ipv6.clone() }); }
}

fn validate_content_authorized(projection: &ServerResourceProjection, content_ref: &ContentRef) -> Result<(), String> {
    let auth = projection.content_sources.get(&content_ref.repo).ok_or_else(|| "content_source_not_authorized".to_string())?;
    if !auth.allowed_paths.iter().any(|p| content_ref.path == *p || content_ref.path.starts_with(&format!("{}/", p.trim_end_matches('/')))) {
        return Err("content_path_not_authorized".into());
    }
    Ok(())
}

fn validate_content_ref(content_ref: &ContentRef) -> Result<(), String> {
    if content_ref.repo.trim().is_empty() { return Err("content_repo_required".into()); }
    if content_ref.commit.trim().is_empty() { return Err("content_commit_required".into()); }
    if content_ref.path.trim().is_empty() { return Err("content_path_required".into()); }
    if content_ref.path.contains("..") { return Err("content_path_parent_traversal".into()); }
    Ok(())
}

fn validate_domain(domain: &str) -> Result<(), String> {
    let domain = domain.trim();
    if domain.is_empty() { return Err("domain_required".into()); }
    if domain.len() > 253 { return Err("domain_too_long".into()); }
    if domain.contains('/') || domain.contains(':') || domain.contains('\0') || domain.chars().any(char::is_whitespace) { return Err("invalid_domain".into()); }
    for label in domain.trim_end_matches('.').split('.') {
        if label.is_empty() || label.len() > 63 || label.starts_with('-') || label.ends_with('-') { return Err("invalid_domain_label".into()); }
    }
    Ok(())
}

fn validate_address(address: &str) -> Result<(), String> {
    let Some((local, domain)) = address.trim().split_once('@') else { return Err("invalid_email_address".into()); };
    if local.is_empty() || local.contains('/') || local.contains('\0') || local.chars().any(char::is_whitespace) { return Err("invalid_email_local_part".into()); }
    validate_domain(domain)
}

fn normalize_domain(domain: &str) -> String {
    domain.trim().trim_end_matches('.').to_ascii_lowercase()
}

fn normalize_address(address: &str) -> String {
    let (local, domain) = address.trim().split_once('@').unwrap_or((address.trim(), ""));
    format!("{}@{}", local, normalize_domain(domain))
}

fn address_domain(address: &str) -> Result<&str, String> {
    address.split_once('@').map(|(_, d)| d).ok_or_else(|| "invalid_email_address".into())
}
