/**
 * Qwen Code Browser Client - Usage Example
 * 
 * This example shows how to use the Qwen Code client in a browser context.
 * 
 * Basic Usage:
 * ```typescript
 * import { QwenCodeClient, QwenOAuth } from '@/lib/qwen/client';
 * 
 * // Initialize with OAuth token
 * const token: QwenToken = {
 *   access_token: 'your-token',
 *   token_type: 'Bearer',
 *   refresh_token: 'your-refresh-token',
 *   resource_url: 'portal.qwen.ai',
 *   expiry_date: Date.now() + 3600000
 * };
 * 
 * const client = QwenCodeClient.fromToken(token);
 * 
 * // Make a chat request
 * const response = await client.chat({
 *   messages: [{ role: 'user', content: 'Hello!' }]
 * });
 * 
 * console.log(response.choices[0].message.content);
 * ```
 * 
 * OAuth Flow:
 * ```typescript
 * // Open OAuth popup
 * const token = await QwenOAuth.openPopup(
 *   'your-client-id',
 *   'http://localhost:4321/api/qwen/callback'
 * );
 * 
 * if (token) {
 *   // Store token and create client
 *   localStorage.setItem('qwen_token', JSON.stringify(token));
 *   const client = QwenCodeClient.fromToken(token);
 * }
 * ```
 * 
 * Streaming:
 * ```typescript
 * const client = QwenCodeClient.fromToken(token);
 * 
 * for await (const chunk of client.chatStream({
 *   messages: [{ role: 'user', content: 'Write a poem' }]
 * })) {
 *   console.log(chunk); // Streamed text chunk
 * }
 * ```
 */

import { QwenCodeClient, QwenOAuth, type QwenToken } from './client';

// Example: Initialize client from stored token
export function createQwenClientFromStorage(): QwenCodeClient | null {
  const tokenStr = localStorage.getItem('qwen_token');
  if (!tokenStr) return null;

  try {
    const token: QwenToken = JSON.parse(tokenStr);
    
    // Check if token is expired
    if (Date.now() > token.expiry_date) {
      console.warn('[Qwen] Token expired, please re-authenticate');
      return null;
    }

    return QwenCodeClient.fromToken(token);
  } catch (error) {
    console.error('[Qwen] Failed to parse token:', error);
    return null;
  }
}

// Example: Start OAuth flow
export async function startQwenOAuth(): Promise<QwenToken | null> {
  const clientId = import.meta.env.QWEN_CLIENT_ID || '';
  const redirectUri = `${window.location.origin}/api/qwen/callback`;

  if (!clientId) {
    console.error('[Qwen] Client ID not configured');
    return null;
  }

  try {
    const token = await QwenOAuth.openPopup(clientId, redirectUri);
    
    if (token) {
      // Store token for future use
      localStorage.setItem('qwen_token', JSON.stringify(token));
      console.log('[Qwen] OAuth successful, token stored');
    } else {
      console.warn('[Qwen] OAuth cancelled or failed');
    }

    return token;
  } catch (error) {
    console.error('[Qwen] OAuth error:', error);
    return null;
  }
}

// Example: Chat with streaming
export async function chatWithQwen(
  client: QwenCodeClient,
  message: string,
  model: string,
  onChunk: (text: string) => void
): Promise<string> {
  let fullResponse = '';

  try {
    for await (const chunk of client.chatStream({
      model,
      messages: [{ role: 'user', content: message }],
    })) {
      fullResponse += chunk;
      onChunk(chunk);
    }
  } catch (error) {
    console.error('[Qwen] Chat error:', error);
    throw error;
  }

  return fullResponse;
}

export { QwenCodeClient, QwenOAuth };
export type { QwenToken, QwenMessage, QwenChatRequest, QwenChatResponse } from './client';
