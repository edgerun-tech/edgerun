globalThis.process ??= {}; globalThis.process.env ??= {};
export { r as renderers } from '../../../chunks/_@astro-renderers_B30lzduo.mjs';

const prerender = false;
const QWEN_CLIENT_ID = "f0304373b74a44d2b584a3fb70ca9e56";
const QWEN_TOKEN_URL = "https://chat.qwen.ai/api/v1/oauth2/token";
const QWEN_GRANT_TYPE = "urn:ietf:params:oauth:grant-type:device_code";
const POST = async ({ request, cookies }) => {
  try {
    const deviceCode = cookies.get("qwen_device_code")?.value;
    const codeVerifier = cookies.get("qwen_code_verifier")?.value;
    if (!deviceCode) {
      return new Response(JSON.stringify({ error: "No device code" }), {
        status: 400,
        headers: { "Content-Type": "application/json" }
      });
    }
    const tokenBody = new URLSearchParams({
      grant_type: QWEN_GRANT_TYPE,
      client_id: QWEN_CLIENT_ID,
      device_code: deviceCode
    });
    if (codeVerifier) {
      tokenBody.set("code_verifier", codeVerifier);
    }
    const tokenResponse = await fetch(QWEN_TOKEN_URL, {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
        "Accept": "application/json"
      },
      body: tokenBody.toString()
    });
    const tokenData = await tokenResponse.json();
    if (!tokenResponse.ok) {
      return new Response(JSON.stringify({
        error: tokenData.error || "authorization_pending",
        error_description: tokenData.error_description || "Please complete authorization"
      }), {
        status: tokenResponse.status,
        headers: { "Content-Type": "application/json" }
      });
    }
    const expiryDate = tokenData.expires_in ? Date.now() + tokenData.expires_in * 1e3 : Date.now() + 3600 * 1e3;
    const tokenPayload = {
      access_token: tokenData.access_token,
      refresh_token: tokenData.refresh_token,
      token_type: tokenData.token_type || "Bearer",
      resource_url: tokenData.resource_url || "chat.qwen.ai",
      expiry_date: expiryDate
    };
    cookies.delete("qwen_device_code", { path: "/" });
    cookies.delete("qwen_code_verifier", { path: "/" });
    return new Response(JSON.stringify(tokenPayload), {
      headers: { "Content-Type": "application/json" }
    });
  } catch (error) {
    console.error("[Qwen Poll] Error:", error);
    return new Response(JSON.stringify({
      error: "server_error",
      error_description: "Failed to poll for token"
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
