globalThis.process ??= {}; globalThis.process.env ??= {};
import crypto from 'crypto';
export { r as renderers } from '../../chunks/_@astro-renderers_B30lzduo.mjs';

const prerender = false;
const QWEN_CLIENT_ID = "f0304373b74a44d2b584a3fb70ca9e56";
const QWEN_OAUTH_BASE_URL = "https://chat.qwen.ai";
const QWEN_DEVICE_CODE_ENDPOINT = `${QWEN_OAUTH_BASE_URL}/api/v1/oauth2/device/code`;
const QWEN_AUTHORIZE_URL = `${QWEN_OAUTH_BASE_URL}/authorize`;
const QWEN_SCOPE = "openid profile email model.completion";
const GET = async ({ redirect, url, cookies }) => {
  try {
    const codeVerifier = crypto.randomBytes(32).toString("base64url");
    const codeChallenge = crypto.createHash("sha256").update(codeVerifier).digest("base64url");
    cookies.set("qwen_code_verifier", codeVerifier, {
      path: "/",
      httpOnly: true,
      secure: url.protocol === "https:",
      maxAge: 900
      // 15 minutes
    });
    const body = `client_id=${QWEN_CLIENT_ID}&scope=${encodeURIComponent(QWEN_SCOPE)}&code_challenge=${codeChallenge}&code_challenge_method=S256`;
    console.log("[Qwen] Requesting device code from:", QWEN_DEVICE_CODE_ENDPOINT);
    const response = await fetch(QWEN_DEVICE_CODE_ENDPOINT, {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
        "Accept": "application/json",
        "x-request-id": crypto.randomUUID()
      },
      body
    });
    console.log("[Qwen] Device code response status:", response.status);
    if (!response.ok) {
      const errorText = await response.text();
      console.error("[Qwen] Device authorization failed:", response.status, errorText);
      return redirect(`/?error=device_auth_failed&status=${response.status}&detail=${encodeURIComponent(errorText)}`);
    }
    const deviceData = await response.json();
    console.log("[Qwen] Device code response:", JSON.stringify(deviceData, null, 2));
    if (!deviceData.device_code || !deviceData.user_code) {
      console.error("[Qwen] Missing device_code or user_code:", deviceData);
      return redirect("/?error=invalid_device_response");
    }
    cookies.set("qwen_device_code", deviceData.device_code, {
      path: "/",
      httpOnly: true,
      secure: url.protocol === "https:",
      maxAge: deviceData.expires_in || 900
    });
    const authUrl = new URL(QWEN_AUTHORIZE_URL);
    authUrl.searchParams.set("user_code", deviceData.user_code);
    authUrl.searchParams.set("client", "qwen-code");
    console.log("[Qwen] Redirecting to:", authUrl.toString());
    return redirect(authUrl.toString());
  } catch (error) {
    console.error("[Qwen Device Flow] Error:", error);
    return redirect("/?error=device_flow_error");
  }
};

const _page = /*#__PURE__*/Object.freeze(/*#__PURE__*/Object.defineProperty({
  __proto__: null,
  GET,
  prerender
}, Symbol.toStringTag, { value: 'Module' }));

const page = () => _page;

export { page };
