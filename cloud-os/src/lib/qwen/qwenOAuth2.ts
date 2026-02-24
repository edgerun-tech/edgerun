type Credentials = {
  access_token?: string;
  refresh_token?: string;
  token_type?: string;
  expiry_date?: number;
};

type DeviceAuthResponse = {
  user_code: string;
  verification_uri: string;
  verification_uri_complete?: string;
};

export class QwenOAuth2Client {
  private credentials: Credentials = {};

  setCredentials(credentials: Credentials): void {
    this.credentials = { ...credentials };
  }

  async requestDeviceAuthorization(): Promise<DeviceAuthResponse> {
    return {
      user_code: 'OPEN-SETTINGS',
      verification_uri: 'https://portal.qwen.ai',
      verification_uri_complete: 'https://portal.qwen.ai',
    };
  }

  async getAccessToken(): Promise<{ token?: string }> {
    return { token: this.credentials.access_token };
  }

  async getContentGenerator(): Promise<{ generateContent: (input: { model: string; messages: Array<{ role: string; content: string }> }) => Promise<any> }> {
    const token = this.credentials.access_token;
    return {
      generateContent: async (input) => {
        const response = await fetch('/api/qwen/chat', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            model: input.model,
            messages: input.messages,
            token,
          }),
        });
        const data = await response.json();
        return {
          id: data.id || `qwen-${Date.now()}`,
          model: data.model || input.model,
          content: data.choices?.[0]?.message?.content || '',
          usage: data.usage || { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
        };
      },
    };
  }
}
