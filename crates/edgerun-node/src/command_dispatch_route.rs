//! Command dispatch route classification.
//!
//! This module is the first step toward shrinking `command_dispatch.rs` into a
//! validation pipeline plus a small router. It contains no side effects.

use edgerun_proto::edgerun::v0::stream::CommandType;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandRoute {
    Control,
    Snapshot,
    ObjectFetch,
    Query,
    Workload,
    Trust,
    Config,
    AppLifecycle,
    Bootstrap,
    UserAuthority,
    ServerResource,
    ReservedUnknown,
    UnsupportedExtension,
}

pub fn classify_command_type(command_type: i32) -> CommandRoute {
    if crate::server_resources::is_server_resource_command(command_type) {
        return CommandRoute::ServerResource;
    }

    match CommandType::from_i32(command_type) {
        Some(CommandType::AddController)
        | Some(CommandType::RemoveController)
        | Some(CommandType::TransferControl) => CommandRoute::Control,
        Some(CommandType::PublishSnapshot) => CommandRoute::Snapshot,
        Some(CommandType::FetchObject) => CommandRoute::ObjectFetch,
        Some(CommandType::Query) => CommandRoute::Query,
        Some(CommandType::ExecuteWorkload) | Some(CommandType::TerminateWorkload) => {
            CommandRoute::Workload
        }
        Some(CommandType::CreateDelegation) | Some(CommandType::CreateRevocation) => {
            CommandRoute::Trust
        }
        Some(CommandType::UpdateConfig) => CommandRoute::Config,
        Some(CommandType::InstallApp) | Some(CommandType::UninstallApp) => {
            CommandRoute::AppLifecycle
        }
        Some(CommandType::CreateIdentity)
        | Some(CommandType::ImportIdentity)
        | Some(CommandType::AddBootstrapNode)
        | Some(CommandType::AddReachabilityHint)
        | Some(CommandType::QueryNodeState) => CommandRoute::Bootstrap,
        Some(CommandType::RequestUserPresence) | Some(CommandType::RequestSignature) => {
            CommandRoute::UserAuthority
        }
        Some(CommandType::Unspecified)
        | Some(CommandType::StoreObject)
        | Some(CommandType::StoreAndForward)
        | Some(CommandType::PutSecret)
        | Some(CommandType::DeleteSecret)
        | Some(CommandType::ListSecrets) => CommandRoute::UnsupportedExtension,
        None if command_type > 0 && command_type < 1000 => CommandRoute::ReservedUnknown,
        None => CommandRoute::UnsupportedExtension,
    }
}

pub fn unsupported_reason(command_type: i32) -> &'static str {
    match classify_command_type(command_type) {
        CommandRoute::ReservedUnknown => "unknown_command_type_reserved",
        CommandRoute::UnsupportedExtension => "unsupported_extension_command_type",
        _ => "unsupported_command_route",
    }
}
