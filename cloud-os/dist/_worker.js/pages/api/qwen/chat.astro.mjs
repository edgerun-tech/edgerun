globalThis.process ??= {}; globalThis.process.env ??= {};
export { r as renderers } from '../../../chunks/_@astro-renderers_B30lzduo.mjs';

const prerender = false;
const QWEN_API_BASE = "https://dashscope.aliyuncs.com/compatible-mode/v1";
const POST = async ({ request, cookies }) => {
  try {
    const body = await request.json();
    const { model, messages, tools, temperature, max_tokens, token } = body;
    let accessToken;
    const qwenTokenStr = cookies.get("qwen_token")?.value || token;
    if (!qwenTokenStr) {
      return new Response(JSON.stringify({
        error: "No authentication token. Please connect Qwen OAuth first."
      }), {
        status: 401,
        headers: { "Content-Type": "application/json" }
      });
    }
    try {
      const tokenData = typeof qwenTokenStr === "string" ? JSON.parse(qwenTokenStr) : qwenTokenStr;
      accessToken = tokenData.access_token;
      if (tokenData.expiry_date && Date.now() > tokenData.expiry_date) {
        return new Response(JSON.stringify({
          error: "Token expired. Please reconnect Qwen OAuth."
        }), {
          status: 401,
          headers: { "Content-Type": "application/json" }
        });
      }
    } catch {
      return new Response(JSON.stringify({
        error: "Invalid token format"
      }), {
        status: 401,
        headers: { "Content-Type": "application/json" }
      });
    }
    const requestBody = {
      model: model || "qwen-plus",
      messages,
      temperature: temperature ?? 0.7,
      max_tokens: max_tokens || 2e3,
      stream: false
    };
    if (tools && tools.length > 0) {
      requestBody.tools = tools;
    }
    const response = await fetch(`${QWEN_API_BASE}/chat/completions`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Authorization": `Bearer ${accessToken}`,
        "X-DashScope-CacheControl": "enable"
      },
      body: JSON.stringify(requestBody),
      signal: AbortSignal.timeout(6e4)
    });
    if (!response.ok) {
      const errorText = await response.text();
      console.error("[Qwen Proxy] API error:", response.status, errorText);
      try {
        const errorData = JSON.parse(errorText);
        return new Response(JSON.stringify({
          error: errorData.error || { message: errorText, type: "api_error" }
        }), {
          status: response.status,
          headers: { "Content-Type": "application/json" }
        });
      } catch {
        return new Response(JSON.stringify({
          error: { message: errorText, type: "api_error" }
        }), {
          status: response.status,
          headers: { "Content-Type": "application/json" }
        });
      }
    }
    const data = await response.json();
    return new Response(JSON.stringify(data), {
      headers: { "Content-Type": "application/json" }
    });
  } catch (error) {
    console.error("[Qwen Proxy] Error:", error);
    if (error.name === "TimeoutError" || error.message?.includes("timeout")) {
      return new Response(JSON.stringify({
        error: { message: "Request timed out", type: "timeout" }
      }), {
        status: 504,
        headers: { "Content-Type": "application/json" }
      });
    }
    return new Response(JSON.stringify({
      error: { message: error.message || "Proxy error", type: "proxy_error" }
    }), {
      status: 500,
      headers: { "Content-Type": "application/json" }
    });
  }
};

const _page = /*#__PURE__*/Object.freeze(/*#__PURE__*/Object.defineProperty({
  __proto__: null,
  POST,
  prerender
}, Symbol.toStringTag, { value: 'Module' }));

const page = () => _page;

export { page };
