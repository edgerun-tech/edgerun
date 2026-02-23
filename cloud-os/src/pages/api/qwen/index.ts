import type { APIRoute } from 'astro';
import crypto from 'crypto';

export const prerender = false;

// Hardcoded Qwen OAuth credentials (from @qwen-code/qwen-code CLI)
const QWEN_CLIENT_ID = 'f0304373b74a44d2b584a3fb70ca9e56';

// Qwen endpoints (matching @qwen-code/qwen-code)
const QWEN_OAUTH_BASE_URL = 'https://chat.qwen.ai';
const QWEN_DEVICE_CODE_ENDPOINT = `${QWEN_OAUTH_BASE_URL}/api/v1/oauth2/device/code`;
const QWEN_TOKEN_ENDPOINT = `${QWEN_OAUTH_BASE_URL}/api/v1/oauth2/token`;
const QWEN_AUTHORIZE_URL = `${QWEN_OAUTH_BASE_URL}/authorize`;

// OAuth constants (from qwen-code CLI)
const QWEN_SCOPE = 'openid profile email model.completion';
const QWEN_GRANT_TYPE = 'urn:ietf:params:oauth:grant-type:device_code';

export const GET: APIRoute = async ({ redirect, url, cookies }) => {
  try {
    // Generate PKCE code verifier and challenge (required by Qwen)
    const codeVerifier = crypto.randomBytes(32).toString('base64url');
    const codeChallenge = crypto.createHash('sha256').update(codeVerifier).digest('base64url');
    
    // Store code verifier for token exchange
    cookies.set('qwen_code_verifier', codeVerifier, {
      path: '/',
      httpOnly: true,
      secure: url.protocol === 'https:',
      maxAge: 900, // 15 minutes
    });

    // Request device code (form-urlencoded with PKCE)
    const body = `client_id=${QWEN_CLIENT_ID}&scope=${encodeURIComponent(QWEN_SCOPE)}&code_challenge=${codeChallenge}&code_challenge_method=S256`;

    console.log('[Qwen] Requesting device code from:', QWEN_DEVICE_CODE_ENDPOINT);
    
    const response = await fetch(QWEN_DEVICE_CODE_ENDPOINT, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
        'Accept': 'application/json',
        'x-request-id': crypto.randomUUID(),
      },
      body,
    });

    console.log('[Qwen] Device code response status:', response.status);
    
    if (!response.ok) {
      const errorText = await response.text();
      console.error('[Qwen] Device authorization failed:', response.status, errorText);
      return redirect(`/?error=device_auth_failed&status=${response.status}&detail=${encodeURIComponent(errorText)}`);
    }

    const deviceData = await response.json();
    console.log('[Qwen] Device code response:', JSON.stringify(deviceData, null, 2));

    if (!deviceData.device_code || !deviceData.user_code) {
      console.error('[Qwen] Missing device_code or user_code:', deviceData);
      return redirect('/?error=invalid_device_response');
    }

    // Store device code for polling
    cookies.set('qwen_device_code', deviceData.device_code, {
      path: '/',
      httpOnly: true,
      secure: url.protocol === 'https:',
      maxAge: deviceData.expires_in || 900,
    });

    // Redirect to Qwen device verification page
    const authUrl = new URL(QWEN_AUTHORIZE_URL);
    authUrl.searchParams.set('user_code', deviceData.user_code);
    authUrl.searchParams.set('client', 'qwen-code');
    
    console.log('[Qwen] Redirecting to:', authUrl.toString());

    return redirect(authUrl.toString());
  } catch (error) {
    console.error('[Qwen Device Flow] Error:', error);
    return redirect('/?error=device_flow_error');
  }
}
