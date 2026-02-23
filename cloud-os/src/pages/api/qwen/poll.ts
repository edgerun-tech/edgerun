import type { APIRoute } from 'astro';

export const prerender = false;

// Hardcoded Qwen OAuth credentials (from @qwen-code/qwen-code CLI)
const QWEN_CLIENT_ID = 'f0304373b74a44d2b584a3fb70ca9e56';
const QWEN_TOKEN_URL = 'https://chat.qwen.ai/api/v1/oauth2/token';
const QWEN_GRANT_TYPE = 'urn:ietf:params:oauth:grant-type:device_code';

export const POST: APIRoute = async ({ request, cookies }) => {
  try {
    // Get device code from cookie
    const deviceCode = cookies.get('qwen_device_code')?.value;
    const codeVerifier = cookies.get('qwen_code_verifier')?.value;

    if (!deviceCode) {
      return new Response(JSON.stringify({ error: 'No device code' }), {
        status: 400,
        headers: { 'Content-Type': 'application/json' }
      });
    }

    // Build token request body
    const tokenBody = new URLSearchParams({
      grant_type: QWEN_GRANT_TYPE,
      client_id: QWEN_CLIENT_ID,
      device_code: deviceCode,
    });
    
    // Add code_verifier if we have it (PKCE)
    if (codeVerifier) {
      tokenBody.set('code_verifier', codeVerifier);
    }

    // Poll for token
    const tokenResponse = await fetch(QWEN_TOKEN_URL, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
        'Accept': 'application/json',
      },
      body: tokenBody.toString(),
    });

    const tokenData = await tokenResponse.json();

    if (!tokenResponse.ok) {
      // Return OAuth error (authorization_pending, slow_down, etc.)
      return new Response(JSON.stringify({
        error: tokenData.error || 'authorization_pending',
        error_description: tokenData.error_description || 'Please complete authorization',
      }), {
        status: tokenResponse.status,
        headers: { 'Content-Type': 'application/json' }
      });
    }

    // Success! Return token
    const expiryDate = tokenData.expires_in
      ? Date.now() + (tokenData.expires_in * 1000)
      : Date.now() + (3600 * 1000);

    const tokenPayload = {
      access_token: tokenData.access_token,
      refresh_token: tokenData.refresh_token,
      token_type: tokenData.token_type || 'Bearer',
      resource_url: tokenData.resource_url || 'chat.qwen.ai',
      expiry_date: expiryDate,
    };

    // Clear device code cookie
    cookies.delete('qwen_device_code', { path: '/' });
    cookies.delete('qwen_code_verifier', { path: '/' });

    return new Response(JSON.stringify(tokenPayload), {
      headers: { 'Content-Type': 'application/json' }
    });
  } catch (error) {
    console.error('[Qwen Poll] Error:', error);
    return new Response(JSON.stringify({ 
      error: 'server_error',
      error_description: 'Failed to poll for token'
    }), {
      status: 500,
      headers: { 'Content-Type': 'application/json' }
    });
  }
}
