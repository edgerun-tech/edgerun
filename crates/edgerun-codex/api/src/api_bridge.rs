use crate::TransportError;
use crate::error::ApiError;
use crate::rate_limits::parse_promo_message;
use crate::rate_limits::parse_rate_limit_for_limit;
use codex_protocol::auth::PlanType;
use codex_protocol::error::CodexErr;
use codex_protocol::error::RetryLimitReachedError;
use codex_protocol::error::UnexpectedResponseError;
use codex_protocol::error::UsageLimitReachedError;
use edgerun_encoding::base64::standard_decode;
use edgerun_http::HeaderMap;
use edgerun_json::JsonValue;
use edgerun_time::chrono::unix_seconds_to_chrono;

pub fn map_api_error(err: ApiError) -> CodexErr {
    match err {
        ApiError::ContextWindowExceeded => CodexErr::ContextWindowExceeded,
        ApiError::QuotaExceeded => CodexErr::QuotaExceeded,
        ApiError::UsageNotIncluded => CodexErr::UsageNotIncluded,
        ApiError::Retryable { message, delay } => CodexErr::Stream(message, delay),
        ApiError::Stream(msg) => CodexErr::Stream(msg, None),
        ApiError::ServerOverloaded => CodexErr::ServerOverloaded,
        ApiError::Api { status, message } => CodexErr::UnexpectedStatus(UnexpectedResponseError {
            status,
            body: message,
            url: None,
            cf_ray: None,
            request_id: None,
            identity_authorization_error: None,
            identity_error_code: None,
        }),
        ApiError::InvalidRequest { message } => CodexErr::InvalidRequest(message),
        ApiError::CyberPolicy { message } => CodexErr::CyberPolicy { message },
        ApiError::Transport(transport) => match transport {
            TransportError::Http {
                status,
                url,
                headers,
                body,
            } => {
                let body_text = body.unwrap_or_default();

                let body_error = body_error_fields(&body_text);

                if status == edgerun_http::StatusCode::SERVICE_UNAVAILABLE
                    && matches!(
                        body_error.as_ref().and_then(|error| error.code.as_deref()),
                        Some("server_is_overloaded" | "slow_down")
                    )
                {
                    return CodexErr::ServerOverloaded;
                }

                if status == edgerun_http::StatusCode::BAD_REQUEST {
                    if let Some(error) = body_error.as_ref()
                        && error.code.as_deref() == Some(CYBER_POLICY_ERROR_CODE)
                    {
                        let message = error
                            .message
                            .as_deref()
                            .filter(|message| !message.trim().is_empty())
                            .map(str::to_string)
                            .unwrap_or_else(|| CYBER_POLICY_FALLBACK_MESSAGE.to_string());
                        CodexErr::CyberPolicy { message }
                    } else if body_text
                        .contains("The image data you provided does not represent a valid image")
                    {
                        CodexErr::InvalidImageRequest()
                    } else {
                        CodexErr::InvalidRequest(body_text)
                    }
                } else if status == edgerun_http::StatusCode::INTERNAL_SERVER_ERROR {
                    CodexErr::InternalServerError
                } else if status == edgerun_http::StatusCode::TOO_MANY_REQUESTS {
                    if let Some(err) = usage_error_from_body(&body_text) {
                        if err.error_type.as_deref() == Some("usage_limit_reached") {
                            let limit_id = extract_header(headers.as_ref(), ACTIVE_LIMIT_HEADER);
                            let rate_limits = headers.as_ref().and_then(|map| {
                                parse_rate_limit_for_limit(map, limit_id.as_deref())
                            });
                            let promo_message = headers.as_ref().and_then(parse_promo_message);
                            let resets_at = err
                                .resets_at
                                .and_then(|seconds| unix_seconds_to_chrono(seconds).ok());
                            return CodexErr::UsageLimitReached(UsageLimitReachedError {
                                plan_type: err.plan_type,
                                resets_at,
                                rate_limits: rate_limits.map(Box::new),
                                promo_message,
                            });
                        } else if err.error_type.as_deref() == Some("usage_not_included") {
                            return CodexErr::UsageNotIncluded;
                        }
                    }

                    CodexErr::RetryLimit(RetryLimitReachedError {
                        status,
                        request_id: extract_request_tracking_id(headers.as_ref()),
                    })
                } else {
                    CodexErr::UnexpectedStatus(UnexpectedResponseError {
                        status,
                        body: body_text,
                        url,
                        cf_ray: extract_header(headers.as_ref(), CF_RAY_HEADER),
                        request_id: extract_request_id(headers.as_ref()),
                        identity_authorization_error: extract_header(
                            headers.as_ref(),
                            X_OPENAI_AUTHORIZATION_ERROR_HEADER,
                        ),
                        identity_error_code: extract_x_error_json_code(headers.as_ref()),
                    })
                }
            }
            TransportError::RetryLimit => CodexErr::RetryLimit(RetryLimitReachedError {
                status: edgerun_http::StatusCode::INTERNAL_SERVER_ERROR,
                request_id: None,
            }),
            TransportError::Timeout => CodexErr::Timeout,
            TransportError::Network(msg) | TransportError::Build(msg) => {
                CodexErr::Stream(msg, None)
            }
        },
        ApiError::RateLimit(msg) => CodexErr::Stream(msg, None),
    }
}

const ACTIVE_LIMIT_HEADER: &str = "x-codex-active-limit";
const REQUEST_ID_HEADER: &str = "x-request-id";
const OAI_REQUEST_ID_HEADER: &str = "x-oai-request-id";
const CF_RAY_HEADER: &str = "cf-ray";
const X_OPENAI_AUTHORIZATION_ERROR_HEADER: &str = "x-openai-authorization-error";
const X_ERROR_JSON_HEADER: &str = "x-error-json";
const CYBER_POLICY_ERROR_CODE: &str = "cyber_policy";
const CYBER_POLICY_FALLBACK_MESSAGE: &str =
    "This request has been flagged for possible cybersecurity risk.";

#[cfg(test)]
#[path = "api_bridge_tests.rs"]
mod tests;

fn extract_request_tracking_id(headers: Option<&HeaderMap>) -> Option<String> {
    extract_request_id(headers).or_else(|| extract_header(headers, CF_RAY_HEADER))
}

fn extract_request_id(headers: Option<&HeaderMap>) -> Option<String> {
    extract_header(headers, REQUEST_ID_HEADER)
        .or_else(|| extract_header(headers, OAI_REQUEST_ID_HEADER))
}

fn extract_header(headers: Option<&HeaderMap>, name: &str) -> Option<String> {
    headers.and_then(|map| {
        map.get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
    })
}

fn extract_x_error_json_code(headers: Option<&HeaderMap>) -> Option<String> {
    let encoded = extract_header(headers, X_ERROR_JSON_HEADER)?;
    let decoded = standard_decode(&encoded).ok()?;
    let parsed = edgerun_json::from_slice(&decoded).ok()?;
    parsed
        .get("error")
        .and_then(|error| error.get("code"))
        .and_then(JsonValue::as_str)
        .map(str::to_string)
}

struct UsageErrorBody {
    error_type: Option<String>,
    plan_type: Option<PlanType>,
    resets_at: Option<i64>,
}

struct BodyErrorFields {
    code: Option<String>,
    message: Option<String>,
}

fn body_error_fields(body: &str) -> Option<BodyErrorFields> {
    let tape = edgerun_json::parse_json_tape(body).ok()?;
    let error = tape.root(body)?.get("error")?;
    Some(BodyErrorFields {
        code: error
            .get("code")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        message: error
            .get("message")
            .and_then(|value| value.as_str())
            .map(str::to_string),
    })
}

fn usage_error_from_body(body: &str) -> Option<UsageErrorBody> {
    let tape = edgerun_json::parse_json_tape(body).ok()?;
    let error = tape.root(body)?.get("error")?;
    let error_type = error
        .get("type")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let plan_type = error
        .get("plan_type")
        .and_then(|value| value.as_str())
        .map(PlanType::from_raw_value);
    let resets_at = error.get("resets_at").and_then(|value| value.as_i64());

    Some(UsageErrorBody {
        error_type,
        plan_type,
        resets_at,
    })
}
