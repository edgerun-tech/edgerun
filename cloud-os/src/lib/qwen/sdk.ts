/**
 * Qwen Code SDK Browser Wrapper
 * Uses the official OAuth flow and API client
 */

import { QwenOAuth2Client } from './qwenOAuth2';

export interface QwenSDKOptions {
  model?: string;
  sessionId?: string;
}

export interface QwenMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

export interface QwenResponse {
  id: string;
  model: string;
  content: string;
  usage: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
}

export class QwenSDK {
  private oauthClient: QwenOAuth2Client;
  private model: string;

  constructor() {
    this.oauthClient = new QwenOAuth2Client();
    this.model = 'qwen-plus';
  }

  /**
   * Initialize with OAuth token from localStorage
   */
  async init(): Promise<boolean> {
    if (typeof window === 'undefined') return false;
    
    const tokenStr = localStorage.getItem('qwen_token');
    if (!tokenStr) return false;

    try {
      const tokenData = JSON.parse(tokenStr);
      this.oauthClient.setCredentials(tokenData);
      this.model = 'qwen-plus';
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Start OAuth device flow
   */
  async startAuth(): Promise<{ userCode: string; verificationUrl: string }> {
    const result = await this.oauthClient.requestDeviceAuthorization({
      scope: 'openid profile email model.completion',
      code_challenge: '',
      code_challenge_method: 'S256',
    });
    
    return {
      userCode: result.user_code,
      verificationUrl: result.verification_uri_complete || result.verification_uri,
    };
  }

  /**
   * Send a chat message
   */
  async chat(messages: QwenMessage[], options?: QwenSDKOptions): Promise<QwenResponse> {
    const tokenResult = await this.oauthClient.getAccessToken();
    
    if (!tokenResult.token) {
      throw new Error('Not authenticated. Please complete OAuth flow.');
    }

    // Use the content generator to make the actual API call
    const contentGenerator = await this.oauthClient.getContentGenerator();
    
    const response = await contentGenerator.generateContent({
      model: options?.model || this.model,
      messages,
    });

    return {
      id: response.id,
      model: response.model,
      content: response.content,
      usage: response.usage,
    };
  }

  /**
   * Check if authenticated
   */
  isAuthenticated(): boolean {
    if (typeof window === 'undefined') return false;
    const tokenStr = localStorage.getItem('qwen_token');
    if (!tokenStr) return false;
    
    try {
      const tokenData = JSON.parse(tokenStr);
      return tokenData.access_token && Date.now() < tokenData.expiry_date;
    } catch {
      return false;
    }
  }
}

export const qwenSDK = new QwenSDK();
