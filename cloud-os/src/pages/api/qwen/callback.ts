import type { APIRoute } from 'astro';

export const prerender = false;

// Hardcoded Qwen OAuth credentials (from @qwen-code/qwen-code CLI)
const QWEN_CLIENT_ID = 'f0304373b74a44d2b584a3fb70ca9e56';
const QWEN_CLIENT_SECRET = 'G5sH7jK9mN2pQ4rT6vX8yA1bC3dE5fH7'; // Placeholder - update with actual secret

const QWEN_TOKEN_URL = 'https://chat.qwen.ai/api/v1/oauth2/token';
const QWEN_GRANT_TYPE = 'urn:ietf:params:oauth:grant-type:device_code';

export const GET: APIRoute = async ({ url, cookies, redirect }) => {
  const error = url.searchParams.get('error');
  const errorDesc = url.searchParams.get('error_description');

  // Handle OAuth errors from Qwen
  if (error) {
    console.error('[Qwen OAuth] Error from provider:', error, errorDesc);
    return redirect(`/?error=${error}`);
  }

  // Get device code and code verifier from cookies
  const deviceCode = cookies.get('qwen_device_code')?.value;
  const codeVerifier = cookies.get('qwen_code_verifier')?.value;
  
  if (!deviceCode) {
    console.error('[Qwen OAuth] No device code in cookie');
    return redirect('/?error=oauth_failed');
  }

  try {
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
    
    // Add client_secret if configured
    if (QWEN_CLIENT_SECRET && QWEN_CLIENT_SECRET !== 'G5sH7jK9mN2pQ4rT6vX8yA1bC3dE5fH7') {
      tokenBody.set('client_secret', QWEN_CLIENT_SECRET);
    }

    // Poll for token (user should have completed auth by now)
    const tokenResponse = await fetch(QWEN_TOKEN_URL, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
        'Accept': 'application/json',
      },
      body: tokenBody.toString(),
    });

    if (!tokenResponse.ok) {
      const errorText = await tokenResponse.text();
      console.error('[Qwen OAuth] Token exchange failed:', tokenResponse.status, errorText);
      
      // Check if it's just pending - show pending page
      if (tokenResponse.status === 400) {
        try {
          const errorData = JSON.parse(errorText);
          if (errorData.error === 'authorization_pending') {
            // Return a page that auto-retries
            return new Response(`
              <!DOCTYPE html>
              <html>
                <head>
                  <meta charset="utf-8">
                  <title>Completing Qwen Auth...</title>
                  <meta http-equiv="refresh" content="3;url=/api/qwen/callback">
                  <meta http-equiv="cache-control" content="no-cache">
                  <style>
                    body { font-family: system-ui; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #1a1a1a; color: #fff; }
                    .loader { text-align: center; }
                    .spinner { border: 4px solid #333; border-top: 4px solid #3498db; border-radius: 50%; width: 40px; height: 40px; animation: spin 1s linear infinite; margin: 0 auto 20px; }
                    @keyframes spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }
                  </style>
                </head>
                <body>
                  <div class="loader">
                    <div class="spinner"></div>
                    <h2>Completing Qwen authentication...</h2>
                    <p>Please wait while we complete your authorization.</p>
                    <p style="color: #888; font-size: 12px; margin-top: 20px;">This page will refresh automatically...</p>
                  </div>
                </body>
              </html>
            `, {
              headers: { 
                'Content-Type': 'text/html',
                'Cache-Control': 'no-cache',
              }
            });
          }
        } catch (e) {
          // Ignore parse error
        }
      }
      
      return redirect(`/?error=token_exchange_failed&detail=${encodeURIComponent(errorText)}`);
    }

    const tokenData = await tokenResponse.json();

    if (!tokenData.access_token) {
      console.error('[Qwen OAuth] No access token received:', tokenData);
      return redirect('/?error=token_exchange_failed');
    }

    // Calculate expiry date
    const expiryDate = tokenData.expires_in
      ? Date.now() + (tokenData.expires_in * 1000)
      : Date.now() + (3600 * 1000);

    // Prepare token payload
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

    // Redirect to frontend with token payload
    const frontendUrl = url.origin;
    const callbackUrl = new URL(`${frontendUrl}/`);
    callbackUrl.searchParams.set('qwen_token', JSON.stringify(tokenPayload));

    console.log('[Qwen OAuth] Success! Redirecting to frontend');

    return redirect(callbackUrl.toString());
  } catch (error) {
    console.error('[Qwen OAuth] Error during token exchange:', error);
    return redirect('/?error=oauth_error');
  }
}
