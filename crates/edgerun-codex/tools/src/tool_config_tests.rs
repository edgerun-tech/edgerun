use super::*;
use crate::Feature;
use crate::Features;
use codex_protocol::compat::absolute_path::AbsolutePathBuf;
use codex_protocol::config_types::WebSearchMode;
use codex_protocol::openai_models::ConfigShellToolType;
use codex_protocol::openai_models::InputModality;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use edgerun_json::json;
use std::path::PathBuf;

fn model_info() -> ModelInfo {
    edgerun_json::from_json_value(json!({
        "slug": "test-model",
        "display_name": "Test Model",
        "description": null,
        "supported_reasoning_levels": [],
        "shell_type": "unified_exec",
        "visibility": "list",
        "supported_in_api": true,
        "priority": 1,
        "availability_nux": null,
        "upgrade": null,
        "base_instructions": "base",
        "model_messages": null,
        "supports_reasoning_summaries": false,
        "default_reasoning_summary": "auto",
        "support_verbosity": false,
        "default_verbosity": null,
        "apply_patch_tool_type": null,
        "truncation_policy": {
            "mode": "bytes",
            "limit": 10000
        },
        "supports_parallel_tool_calls": false,
        "supports_image_detail_original": false,
        "context_window": null,
        "auto_compact_token_limit": null,
        "effective_context_window_percent": 95,
        "experimental_supported_tools": [],
        "input_modalities": ["text", "image"],
        "supports_search_tool": false
    }))
    .expect("deserialize test model")
}

#[test]
fn model_provided_unified_exec_requires_feature_flag() {
    let model_info = model_info();
    let mut features = Features::with_defaults();
    features.disable(Feature::UnifiedExec);

    let available_models = Vec::new();
    let tools_config = ToolsConfig::new(&ToolsConfigParams {
        model_info: &model_info,
        available_models: &available_models,
        features: &features,
        image_generation_tool_auth_allowed: true,
        web_search_mode: Some(WebSearchMode::Cached),
        session_source: SessionSource::Cli,
    });

    assert_eq!(tools_config.shell_type, ConfigShellToolType::ShellCommand);
}

#[test]
fn unified_exec_can_be_enabled_for_workspace_write() {
    let model_info = model_info();
    let mut features = Features::with_defaults();
    features.enable(Feature::UnifiedExec);

    let available_models = Vec::new();
    let tools_config = ToolsConfig::new(&ToolsConfigParams {
        model_info: &model_info,
        available_models: &available_models,
        features: &features,
        image_generation_tool_auth_allowed: true,
        web_search_mode: Some(WebSearchMode::Cached),
        session_source: SessionSource::Cli,
    });

    let expected_shell_type = if super::conpty_supported() {
        ConfigShellToolType::UnifiedExec
    } else {
        ConfigShellToolType::ShellCommand
    };
    assert_eq!(tools_config.shell_type, expected_shell_type);
}

#[test]
fn shell_zsh_fork_prefers_shell_command_over_unified_exec() {
    let model_info = model_info();
    let mut features = Features::with_defaults();
    features.enable(Feature::UnifiedExec);
    features.enable(Feature::ShellZshFork);

    let available_models = Vec::new();
    let tools_config = ToolsConfig::new(&ToolsConfigParams {
        model_info: &model_info,
        available_models: &available_models,
        features: &features,
        image_generation_tool_auth_allowed: true,
        web_search_mode: Some(WebSearchMode::Live),
        session_source: SessionSource::Cli,
    });

    assert_eq!(tools_config.shell_type, ConfigShellToolType::ShellCommand);
    assert_eq!(
        tools_config.shell_command_backend,
        ShellCommandBackendConfig::ZshFork
    );
    assert_eq!(
        tools_config.unified_exec_shell_mode,
        UnifiedExecShellMode::Direct
    );
    assert_eq!(
        tools_config
            .with_unified_exec_shell_mode_for_session(
                ToolUserShellType::Zsh,
                Some(&PathBuf::from("/opt/codex/zsh")),
                Some(&PathBuf::from("/opt/codex/codex-execve-wrapper")),
            )
            .unified_exec_shell_mode,
        if cfg!(unix) {
            UnifiedExecShellMode::ZshFork(ZshForkConfig {
                shell_zsh_path: AbsolutePathBuf::from_absolute_path("/opt/codex/zsh").unwrap(),
                main_execve_wrapper_exe: AbsolutePathBuf::from_absolute_path(
                    "/opt/codex/codex-execve-wrapper",
                )
                .unwrap(),
            })
        } else {
            UnifiedExecShellMode::Direct
        }
    );
}

#[test]
fn subagents_keep_request_user_input_config_and_agent_jobs_workers_opt_in_by_label() {
    let model_info = model_info();
    let mut features = Features::with_defaults();
    features.enable(Feature::DefaultModeRequestUserInput);
    features.enable(Feature::SpawnCsv);

    let available_models = Vec::new();
    let tools_config = ToolsConfig::new(&ToolsConfigParams {
        model_info: &model_info,
        available_models: &available_models,
        features: &features,
        image_generation_tool_auth_allowed: true,
        web_search_mode: Some(WebSearchMode::Cached),
        session_source: SessionSource::SubAgent(SubAgentSource::Other(
            "agent_job:test".to_string(),
        )),
    });

    assert_eq!(
        tools_config.request_user_input_available_modes,
        request_user_input_available_modes(&features)
    );
    assert!(tools_config.agent_jobs_tools);
    assert!(tools_config.agent_jobs_worker_tools);
}

#[test]
fn image_generation_requires_feature_and_supported_model() {
    let supported_model_info = model_info();
    let mut unsupported_model_info = supported_model_info.clone();
    unsupported_model_info.input_modalities = vec![InputModality::Text];

    let mut image_generation_disabled_features = Features::with_defaults();
    image_generation_disabled_features.disable(Feature::ImageGeneration);
    let mut image_generation_features = Features::with_defaults();
    image_generation_features.enable(Feature::ImageGeneration);

    let available_models = Vec::new();
    let default_tools_config = ToolsConfig::new(&ToolsConfigParams {
        model_info: &supported_model_info,
        available_models: &available_models,
        features: &image_generation_disabled_features,
        image_generation_tool_auth_allowed: true,
        web_search_mode: Some(WebSearchMode::Cached),
        session_source: SessionSource::Cli,
    });
    let supported_tools_config = ToolsConfig::new(&ToolsConfigParams {
        model_info: &supported_model_info,
        available_models: &available_models,
        features: &image_generation_features,
        image_generation_tool_auth_allowed: true,
        web_search_mode: Some(WebSearchMode::Cached),
        session_source: SessionSource::Cli,
    });
    let auth_disallowed_tools_config = ToolsConfig::new(&ToolsConfigParams {
        model_info: &supported_model_info,
        available_models: &available_models,
        features: &image_generation_features,
        image_generation_tool_auth_allowed: false,
        web_search_mode: Some(WebSearchMode::Cached),
        session_source: SessionSource::Cli,
    });
    let unsupported_tools_config = ToolsConfig::new(&ToolsConfigParams {
        model_info: &unsupported_model_info,
        available_models: &available_models,
        features: &image_generation_features,
        image_generation_tool_auth_allowed: true,
        web_search_mode: Some(WebSearchMode::Cached),
        session_source: SessionSource::Cli,
    });
    assert!(!default_tools_config.image_gen_tool);
    assert!(supported_tools_config.image_gen_tool);
    assert!(!auth_disallowed_tools_config.image_gen_tool);
    assert!(!unsupported_tools_config.image_gen_tool);
}

#[test]
fn provider_capability_methods_disable_provider_bound_tool_surfaces() {
    let model_info = model_info();
    let features = Features::with_defaults();
    let available_models = Vec::new();
    let mut tools_config = ToolsConfig::new(&ToolsConfigParams {
        model_info: &model_info,
        available_models: &available_models,
        features: &features,
        image_generation_tool_auth_allowed: true,
        web_search_mode: Some(WebSearchMode::Cached),
        session_source: SessionSource::Cli,
    });
    tools_config.search_tool = true;
    tools_config.tool_suggest = true;
    tools_config.image_gen_tool = true;
    tools_config.namespace_tools = true;

    let tools_config = tools_config
        .with_namespace_tools_capability(/*namespace_tools*/ false)
        .with_image_generation_capability(/*image_generation*/ false)
        .with_web_search_capability(/*web_search*/ false);

    assert!(tools_config.search_tool);
    assert!(tools_config.tool_suggest);
    assert!(!tools_config.image_gen_tool);
    assert!(!tools_config.namespace_tools);
    assert_eq!(tools_config.web_search_mode, None);
}
