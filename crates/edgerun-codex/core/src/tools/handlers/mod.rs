pub(crate) mod agent_jobs;
pub(crate) mod agent_jobs_spec;
mod dynamic;
mod goal;
pub(crate) mod goal_spec;
mod mcp;
mod mcp_resource;
pub(crate) mod mcp_resource_spec;
pub(crate) mod multi_agents;
pub(crate) mod multi_agents_common;
pub(crate) mod multi_agents_spec;
pub(crate) mod multi_agents_v2;
mod plan;
pub(crate) mod plan_spec;
mod request_plugin_install;
pub(crate) mod request_plugin_install_spec;
mod request_user_input;
pub(crate) mod request_user_input_spec;
mod test_sync;
pub(crate) mod test_sync_spec;
mod tool_search;
pub(crate) mod tool_search_spec;
mod unavailable_tool;
mod view_image;
pub(crate) mod view_image_spec;

use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_absolute_path::AbsolutePathBufGuard;
use edgerun_json::FromJson;
use edgerun_json::Value;

use crate::function_tool::FunctionCallError;
use crate::session::turn_context::TurnContext;
use crate::session::turn_context::TurnEnvironment;
pub(crate) use crate::tools::code_mode::CodeModeExecuteHandler;
pub(crate) use crate::tools::code_mode::CodeModeWaitHandler;
pub use dynamic::DynamicToolHandler;
pub use goal::CreateGoalHandler;
pub use goal::GetGoalHandler;
pub use goal::UpdateGoalHandler;
pub use mcp::McpHandler;
pub use mcp_resource::ListMcpResourceTemplatesHandler;
pub use mcp_resource::ListMcpResourcesHandler;
pub use mcp_resource::ReadMcpResourceHandler;
pub use plan::PlanHandler;
pub use request_plugin_install::RequestPluginInstallHandler;
pub use request_user_input::RequestUserInputHandler;
pub use test_sync::TestSyncHandler;
pub use tool_search::ToolSearchHandler;
pub use unavailable_tool::UnavailableToolHandler;
pub(crate) use unavailable_tool::unavailable_tool_message;
pub use view_image::ViewImageHandler;

fn parse_json_arguments<T>(arguments: &str) -> Result<T, FunctionCallError>
where
    T: FromJson,
{
    edgerun_json::from_json_str(arguments).map_err(|err| {
        FunctionCallError::RespondToModel(format!("failed to parse function arguments: {err}"))
    })
}

fn parse_json_arguments_with_base_path<T>(
    arguments: &str,
    base_path: &AbsolutePathBuf,
) -> Result<T, FunctionCallError>
where
    T: FromJson,
{
    let _guard = AbsolutePathBufGuard::new(base_path);
    parse_json_arguments(arguments)
}

fn resolve_workdir_base_path(
    arguments: &str,
    default_cwd: &AbsolutePathBuf,
) -> Result<AbsolutePathBuf, FunctionCallError> {
    let tape = edgerun_json::parse_json_tape(arguments).map_err(|err| {
        FunctionCallError::RespondToModel(format!("failed to parse function arguments: {err}"))
    })?;
    let root = tape.root(arguments);
    Ok(root
        .and_then(|root| root.get("workdir"))
        .and_then(|workdir| workdir.as_str())
        .filter(|workdir| !workdir.is_empty())
        .map_or_else(|| default_cwd.clone(), |workdir| default_cwd.join(workdir)))
}

fn resolve_tool_environment<'a>(
    turn: &'a TurnContext,
    environment_id: Option<&str>,
) -> Result<Option<&'a TurnEnvironment>, FunctionCallError> {
    environment_id.map_or_else(
        || Ok(turn.environments.primary()),
        |environment_id| {
            turn.environments
                .turn_environments
                .iter()
                .find(|environment| environment.environment_id == environment_id)
                .map(Some)
                .ok_or_else(|| {
                    FunctionCallError::RespondToModel(format!(
                        "unknown turn environment id `{environment_id}`"
                    ))
                })
        },
    )
}
