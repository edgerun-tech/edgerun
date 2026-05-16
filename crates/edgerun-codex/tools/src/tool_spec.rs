use crate::FreeformTool;
use crate::JsonSchema;
use crate::LoadableToolSpec;
use crate::ResponsesApiNamespace;
use crate::ResponsesApiTool;
use codex_protocol::config_types::WebSearchContextSize;
use codex_protocol::config_types::WebSearchFilters as ConfigWebSearchFilters;
use codex_protocol::config_types::WebSearchUserLocation as ConfigWebSearchUserLocation;
use codex_protocol::config_types::WebSearchUserLocationType;
use edgerun_json::{Map, ToJson, Value};

/// When serialized as JSON, this produces a valid "Tool" in the OpenAI
/// Responses API.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolSpec {
    Function(ResponsesApiTool),
    Namespace(ResponsesApiNamespace),
    ToolSearch {
        execution: String,
        description: String,
        parameters: JsonSchema,
    },
    LocalShell {},
    ImageGeneration {
        output_format: String,
    },
    // TODO: Understand why we get an error on web_search although the API docs
    // say it's supported.
    // https://platform.openai.com/docs/guides/tools-web-search?api-mode=responses#:~:text=%7B%20type%3A%20%22web_search%22%20%7D%2C
    // The `external_web_access` field determines whether the web search is over
    // cached or live content.
    // https://platform.openai.com/docs/guides/tools-web-search#live-internet-access
    WebSearch {
        external_web_access: Option<bool>,
        filters: Option<ResponsesApiWebSearchFilters>,
        user_location: Option<ResponsesApiWebSearchUserLocation>,
        search_context_size: Option<WebSearchContextSize>,
        search_content_types: Option<Vec<String>>,
    },
    Freeform(FreeformTool),
}

impl ToolSpec {
    pub fn name(&self) -> &str {
        match self {
            ToolSpec::Function(tool) => tool.name.as_str(),
            ToolSpec::Namespace(namespace) => namespace.name.as_str(),
            ToolSpec::ToolSearch { .. } => "tool_search",
            ToolSpec::LocalShell {} => "local_shell",
            ToolSpec::ImageGeneration { .. } => "image_generation",
            ToolSpec::WebSearch { .. } => "web_search",
            ToolSpec::Freeform(tool) => tool.name.as_str(),
        }
    }
}

impl ToJson for ToolSpec {
    fn to_json(&self) -> Value {
        match self {
            Self::Function(tool) => tool.to_json(),
            Self::Namespace(namespace) => namespace.to_json(),
            Self::ToolSearch {
                execution,
                description,
                parameters,
            } => {
                let mut object = Map::new();
                object.push_field("type", "tool_search");
                object.push_field("execution", execution.as_str());
                object.push_field("description", description.as_str());
                object.push_field("parameters", parameters.to_json());
                object.into()
            }
            Self::LocalShell {} => {
                let mut object = Map::new();
                object.push_field("type", "local_shell");
                object.into()
            }
            Self::ImageGeneration { output_format } => {
                let mut object = Map::new();
                object.push_field("type", "image_generation");
                object.push_field("output_format", output_format.as_str());
                object.into()
            }
            Self::WebSearch {
                external_web_access,
                filters,
                user_location,
                search_context_size,
                search_content_types,
            } => {
                let mut object = Map::new();
                object.push_field("type", "web_search");
                object.push_opt_field("external_web_access", *external_web_access);
                object.push_opt_field("filters", filters.as_ref().map(ToJson::to_json));
                object.push_opt_field("user_location", user_location.as_ref().map(ToJson::to_json));
                object.push_opt_field(
                    "search_context_size",
                    search_context_size
                        .as_ref()
                        .map(web_search_context_size_json),
                );
                object.push_opt_field(
                    "search_content_types",
                    search_content_types.as_ref().map(ToJson::to_json),
                );
                object.into()
            }
            Self::Freeform(tool) => tool.to_json(),
        }
    }
}

impl From<LoadableToolSpec> for ToolSpec {
    fn from(value: LoadableToolSpec) -> Self {
        match value {
            LoadableToolSpec::Function(tool) => ToolSpec::Function(tool),
            LoadableToolSpec::Namespace(namespace) => ToolSpec::Namespace(namespace),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfiguredToolSpec {
    pub spec: ToolSpec,
    pub supports_parallel_tool_calls: bool,
}

impl ConfiguredToolSpec {
    pub fn new(spec: ToolSpec, supports_parallel_tool_calls: bool) -> Self {
        Self {
            spec,
            supports_parallel_tool_calls,
        }
    }

    pub fn name(&self) -> &str {
        self.spec.name()
    }
}

/// Returns JSON values that are compatible with Function Calling in the
/// Responses API:
/// https://platform.openai.com/docs/guides/function-calling?api-mode=responses
pub fn create_tools_json_for_responses_api(
    tools: &[ToolSpec],
) -> Result<Vec<Value>, edgerun_json::Error> {
    let mut tools_json = Vec::new();

    for tool in tools {
        let json = tool.to_json();
        tools_json.push(json);
    }

    Ok(tools_json)
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResponsesApiWebSearchFilters {
    pub allowed_domains: Option<Vec<String>>,
}

impl ToJson for ResponsesApiWebSearchFilters {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_opt_field(
            "allowed_domains",
            self.allowed_domains.as_ref().map(ToJson::to_json),
        );
        object.into()
    }
}

impl From<ConfigWebSearchFilters> for ResponsesApiWebSearchFilters {
    fn from(filters: ConfigWebSearchFilters) -> Self {
        Self {
            allowed_domains: filters.allowed_domains,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResponsesApiWebSearchUserLocation {
    pub r#type: WebSearchUserLocationType,
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub timezone: Option<String>,
}

impl ToJson for ResponsesApiWebSearchUserLocation {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("type", web_search_user_location_type_json(&self.r#type));
        object.push_opt_field("country", self.country.as_ref().map(ToJson::to_json));
        object.push_opt_field("region", self.region.as_ref().map(ToJson::to_json));
        object.push_opt_field("city", self.city.as_ref().map(ToJson::to_json));
        object.push_opt_field("timezone", self.timezone.as_ref().map(ToJson::to_json));
        object.into()
    }
}

fn web_search_context_size_json(value: &WebSearchContextSize) -> Value {
    let value = match value {
        WebSearchContextSize::Low => "low",
        WebSearchContextSize::Medium => "medium",
        WebSearchContextSize::High => "high",
    };
    Value::String(value.to_string())
}

fn web_search_user_location_type_json(value: &WebSearchUserLocationType) -> Value {
    match value {
        WebSearchUserLocationType::Approximate => Value::String("approximate".to_string()),
    }
}

impl From<ConfigWebSearchUserLocation> for ResponsesApiWebSearchUserLocation {
    fn from(user_location: ConfigWebSearchUserLocation) -> Self {
        Self {
            r#type: user_location.r#type,
            country: user_location.country,
            region: user_location.region,
            city: user_location.city,
            timezone: user_location.timezone,
        }
    }
}

#[cfg(test)]
#[path = "tool_spec_tests.rs"]
mod tests;
