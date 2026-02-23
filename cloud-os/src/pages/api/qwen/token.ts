import type { APIRoute } from 'astro';

export const prerender = false;

const QWEN_CLIENT_ID = import.meta.env.QWEN_CLIENT_ID || '';
const QWEN_CLIENT_SECRET = import.meta.env.QWEN_CLIENT_SECRET || '';
const QWEN_TOKEN_URI = import.meta.env.QWEN_TOKEN_URI || 'https://portal.qwen.ai/oauth/token';
const QWEN_REDIRECT_URI = import.meta.env.QWEN_REDIRECT_URI || 'http://localhost:4321/api/qwen/callback';

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const { code, grant_type = 'authorization_code', refresh_token } = body;

    if (grant_type === 'authorization_code' && !code) {
      return new Response(JSON.stringify({ error: 'Authorization code required' }), {
        status: 400,
        headers: { 'Content-Type': 'application/json' },
      });
    }

    if (grant_type === 'refresh_token' && !refresh_token) {
      return new Response(JSON.stringify({ error: 'Refresh token required' }), {
        status: 400,
        headers: { 'Content-Type': 'application/json' },
      });
    }

    const params = new URLSearchParams({
      grant_type,
      client_id: QWEN_CLIENT_ID,
      client_secret: QWEN_CLIENT_SECRET,
    });

    if (grant_type === 'authorization_code') {
      params.set('code', code);
      params.set('redirect_uri', QWEN_REDIRECT_URI);
    } else if (grant_type === 'refresh_token') {
      params.set('refresh_token', refresh_token);
    }

    const response = await fetch(QWEN_TOKEN_URI, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
        'Accept': 'application/json',
      },
      body: params,
    });

    if (!response.ok) {
      const errorText = await response.text();
      return new Response(JSON.stringify({ 
        error: 'Token exchange failed',
        details: errorText 
      }), {
        status: response.status,
        headers: { 'Content-Type': 'application/json' },
      });
    }

    const data = await response.json();
    
    // Return token with calculated expiry
    const tokenData = {
      access_token: data.access_token,
      token_type: data.token_type || 'Bearer',
      refresh_token: data.refresh_token,
      resource_url: data.resource_url || 'portal.qwen.ai',
      expiry_date: Date.now() + (data.expires_in || 3600) * 1000,
    };

    return new Response(JSON.stringify(tokenData), {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    });
  } catch (error) {
    console.error('[Qwen Token API] Error:', error);
    return new Response(JSON.stringify({ 
      error: 'Internal server error',
      message: error instanceof Error ? error.message : 'Unknown error'
    }), {
      status: 500,
      headers: { 'Content-Type': 'application/json' },
    });
  }
};
