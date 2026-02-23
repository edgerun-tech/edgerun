import type { APIRoute } from 'astro';

export const prerender = false;

// Use DashScope API (OAuth tokens work here)
const QWEN_API_BASE = 'https://dashscope.aliyuncs.com/compatible-mode/v1';

export const POST: APIRoute = async ({ request, cookies }) => {
  try {
    const body = await request.json();
    const { model, messages, tools, temperature, max_tokens, token } = body;

    // Get OAuth token from cookie or request body
    let accessToken: string;
    const qwenTokenStr = cookies.get('qwen_token')?.value || token;
    
    if (!qwenTokenStr) {
      return new Response(JSON.stringify({ 
        error: 'No authentication token. Please connect Qwen OAuth first.' 
      }), {
        status: 401,
        headers: { 'Content-Type': 'application/json' }
      });
    }

    try {
      const tokenData = typeof qwenTokenStr === 'string' ? JSON.parse(qwenTokenStr) : qwenTokenStr;
      accessToken = tokenData.access_token;
      
      // Check if token is expired
      if (tokenData.expiry_date && Date.now() > tokenData.expiry_date) {
        return new Response(JSON.stringify({ 
          error: 'Token expired. Please reconnect Qwen OAuth.' 
        }), {
          status: 401,
          headers: { 'Content-Type': 'application/json' }
        });
      }
    } catch {
      return new Response(JSON.stringify({ 
        error: 'Invalid token format' 
      }), {
        status: 401,
        headers: { 'Content-Type': 'application/json' }
      });
    }

    // Build request body for DashScope API
    const requestBody: any = {
      model: model || 'qwen-plus',
      messages,
      temperature: temperature ?? 0.7,
      max_tokens: max_tokens || 2000,
      stream: false,
    };
    
    // Add tools if provided
    if (tools && tools.length > 0) {
      requestBody.tools = tools;
    }

    // Call DashScope API
    const response = await fetch(`${QWEN_API_BASE}/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${accessToken}`,
        'X-DashScope-CacheControl': 'enable',
      },
      body: JSON.stringify(requestBody),
      signal: AbortSignal.timeout(60000),
    });

    if (!response.ok) {
      const errorText = await response.text();
      console.error('[Qwen Proxy] API error:', response.status, errorText);
      
      // Try to parse error response
      try {
        const errorData = JSON.parse(errorText);
        return new Response(JSON.stringify({
          error: errorData.error || { message: errorText, type: 'api_error' }
        }), {
          status: response.status,
          headers: { 'Content-Type': 'application/json' }
        });
      } catch {
        return new Response(JSON.stringify({
          error: { message: errorText, type: 'api_error' }
        }), {
          status: response.status,
          headers: { 'Content-Type': 'application/json' }
        });
      }
    }

    const data = await response.json();
    return new Response(JSON.stringify(data), {
      headers: { 'Content-Type': 'application/json' }
    });
  } catch (error: any) {
    console.error('[Qwen Proxy] Error:', error);
    if (error.name === 'TimeoutError' || error.message?.includes('timeout')) {
      return new Response(JSON.stringify({ 
        error: { message: 'Request timed out', type: 'timeout' }
      }), {
        status: 504,
        headers: { 'Content-Type': 'application/json' }
      });
    }
    return new Response(JSON.stringify({ 
      error: { message: error.message || 'Proxy error', type: 'proxy_error' }
    }), {
      status: 500,
      headers: { 'Content-Type': 'application/json' }
    });
  }
}
