//! Shared tool definitions and Responses API tool primitives that can live
//! outside `codex-core`.

extern crate serde as edgerun_serde;

mod code_mode;
mod dynamic_tool;
mod features;
mod image_detail;
mod json_schema;
#[cfg(feature = "mcp")]
mod mcp_tool;
#[cfg(feature = "mcp")]
mod request_plugin_install;
mod responses_api;
mod tool_config;
mod tool_definition;
#[cfg(feature = "mcp")]
mod tool_discovery;
mod tool_spec;

pub use code_mode::augment_tool_spec_for_code_mode;
pub use code_mode::code_mode_name_for_tool_name;
pub use code_mode::collect_code_mode_exec_prompt_tool_definitions;
pub use code_mode::collect_code_mode_tool_definitions;
pub use code_mode::tool_spec_to_code_mode_tool_definition;
pub use codex_protocol::ToolName;
pub use dynamic_tool::parse_dynamic_tool;
pub use features::Feature;
pub use features::Features;
pub use image_detail::can_request_original_image_detail;
pub use image_detail::normalize_output_image_detail;
pub use image_detail::sanitize_original_image_detail;
pub use json_schema::AdditionalProperties;
pub use json_schema::JsonSchema;
pub use json_schema::JsonSchemaPrimitiveType;
pub use json_schema::JsonSchemaType;
pub use json_schema::parse_tool_input_schema;
#[cfg(feature = "mcp")]
pub use mcp_tool::mcp_call_tool_result_output_schema;
#[cfg(feature = "mcp")]
pub use mcp_tool::parse_mcp_tool;
#[cfg(feature = "mcp")]
pub use request_plugin_install::REQUEST_PLUGIN_INSTALL_APPROVAL_KIND_VALUE;
#[cfg(feature = "mcp")]
pub use request_plugin_install::REQUEST_PLUGIN_INSTALL_PERSIST_ALWAYS_VALUE;
#[cfg(feature = "mcp")]
pub use request_plugin_install::REQUEST_PLUGIN_INSTALL_PERSIST_KEY;
#[cfg(feature = "mcp")]
pub use request_plugin_install::RequestPluginInstallArgs;
#[cfg(feature = "mcp")]
pub use request_plugin_install::RequestPluginInstallMeta;
#[cfg(feature = "mcp")]
pub use request_plugin_install::RequestPluginInstallResult;
#[cfg(feature = "mcp")]
pub use request_plugin_install::all_requested_connectors_picked_up;
#[cfg(feature = "mcp")]
pub use request_plugin_install::build_request_plugin_install_elicitation_request;
#[cfg(feature = "mcp")]
pub use request_plugin_install::verified_connector_install_completed;
pub use responses_api::FreeformTool;
pub use responses_api::FreeformToolFormat;
pub use responses_api::LoadableToolSpec;
pub use responses_api::ResponsesApiNamespace;
pub use responses_api::ResponsesApiNamespaceTool;
pub use responses_api::ResponsesApiTool;
pub use responses_api::coalesce_loadable_tool_specs;
pub use responses_api::default_namespace_description;
pub use responses_api::dynamic_tool_to_loadable_tool_spec;
pub use responses_api::dynamic_tool_to_responses_api_tool;
#[cfg(feature = "mcp")]
pub use responses_api::mcp_tool_to_deferred_responses_api_tool;
#[cfg(feature = "mcp")]
pub use responses_api::mcp_tool_to_responses_api_tool;
pub use responses_api::tool_definition_to_responses_api_tool;
pub use tool_config::ShellCommandBackendConfig;
pub use tool_config::ToolEnvironmentMode;
pub use tool_config::ToolUserShellType;
pub use tool_config::ToolsConfig;
pub use tool_config::ToolsConfigParams;
pub use tool_config::UnifiedExecShellMode;
pub use tool_config::ZshForkConfig;
pub use tool_config::request_user_input_available_modes;
pub use tool_definition::ToolDefinition;
#[cfg(feature = "mcp")]
pub use tool_discovery::DiscoverablePluginInfo;
#[cfg(feature = "mcp")]
pub use tool_discovery::DiscoverableTool;
#[cfg(feature = "mcp")]
pub use tool_discovery::DiscoverableToolAction;
#[cfg(feature = "mcp")]
pub use tool_discovery::DiscoverableToolType;
#[cfg(feature = "mcp")]
pub use tool_discovery::REQUEST_PLUGIN_INSTALL_TOOL_NAME;
#[cfg(feature = "mcp")]
pub use tool_discovery::RequestPluginInstallEntry;
#[cfg(feature = "mcp")]
pub use tool_discovery::TOOL_SEARCH_DEFAULT_LIMIT;
#[cfg(feature = "mcp")]
pub use tool_discovery::TOOL_SEARCH_TOOL_NAME;
#[cfg(feature = "mcp")]
pub use tool_discovery::ToolSearchResultSource;
#[cfg(feature = "mcp")]
pub use tool_discovery::ToolSearchSource;
#[cfg(feature = "mcp")]
pub use tool_discovery::ToolSearchSourceInfo;
#[cfg(feature = "mcp")]
pub use tool_discovery::collect_request_plugin_install_entries;
#[cfg(feature = "mcp")]
pub use tool_discovery::collect_tool_search_source_infos;
#[cfg(feature = "mcp")]
pub use tool_discovery::filter_request_plugin_install_discoverable_tools_for_client;
#[cfg(feature = "mcp")]
pub use tool_discovery::tool_search_result_source_to_loadable_tool_spec;
pub use tool_spec::ConfiguredToolSpec;
pub use tool_spec::ResponsesApiWebSearchFilters;
pub use tool_spec::ResponsesApiWebSearchUserLocation;
pub use tool_spec::ToolSpec;
pub use tool_spec::create_tools_json_for_responses_api;
