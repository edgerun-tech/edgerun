import { createSignal, createEffect } from 'solid-js';
import { llmRouter } from '../lib/llm/router';

export type IntegrationStatus = 'connected' | 'disconnected' | 'loading';

export interface IntegrationState {
  id: string;
  name: string;
  status: IntegrationStatus;
  user?: {
    login: string;
    avatar_url: string;
  };
  lastSync?: string;
}

export interface QwenToken {
  access_token: string;
  refresh_token?: string;
  token_type: string;
  resource_url: string;
  expiry_date: number;
}

const [integrations, setIntegrations] = createSignal<IntegrationState[]>([
  { id: 'github', name: 'GitHub', status: 'disconnected' },
  { id: 'google', name: 'Google', status: 'disconnected' },
  { id: 'cloudflare', name: 'Cloudflare', status: 'disconnected' },
  { id: 'llm', name: 'AI Provider', status: 'disconnected' },
]);

const [isInitialized, setIsInitialized] = createSignal(false);

export const integrationStore = {
  integrations,
  
  getIntegration: (id: string) => {
    return integrations().find(i => i.id === id);
  },
  
  isConnected: (id: string) => {
    return integrations().find(i => i.id === id)?.status === 'connected';
  },
  
  connectGitHub: async (token: string) => {
    localStorage.setItem('github_token', token);
    setIsInitialized(false);
    await integrationStore.checkGitHub();
  },
  
  connectGoogle: async (token: string, refreshToken?: string) => {
    localStorage.setItem('google_token', token);
    if (refreshToken) localStorage.setItem('google_refresh', refreshToken);
    setIsInitialized(false);
    await integrationStore.checkGoogle();
  },
  
  connectCloudflare: async (token: string) => {
    localStorage.setItem('cloudflare_token', token);
    setIsInitialized(false);
    await integrationStore.checkCloudflare();
  },

  connectQwen: async (tokenData: QwenToken) => {
    localStorage.setItem('qwen_token', JSON.stringify(tokenData));
    if (tokenData.refresh_token) {
      localStorage.setItem('qwen_refresh', tokenData.refresh_token);
    }
    setIsInitialized(false);
    await integrationStore.checkQwen();
  },
  
  disconnect: async (id: string) => {
    const tokenKeys: Record<string, string> = {
      github: 'github_token',
      google: 'google_token',
      cloudflare: 'cloudflare_token',
      qwen: 'qwen_token',
    };

    localStorage.removeItem(tokenKeys[id]);
    if (id === 'google') {
      localStorage.removeItem('google_refresh');
    }
    if (id === 'qwen') {
      localStorage.removeItem('qwen_refresh');
    }

    setIntegrations(prev => prev.map(i =>
      i.id === id ? { ...i, status: 'disconnected', user: undefined, lastSync: undefined } : i
    ));
  },
  
  checkGitHub: async () => {
    const token = localStorage.getItem('github_token');
    if (!token) {
      setIntegrations(prev => prev.map(i => 
        i.id === 'github' ? { ...i, status: 'disconnected', user: undefined } : i
      ));
      return;
    }
    
    setIntegrations(prev => prev.map(i => 
      i.id === 'github' ? { ...i, status: 'loading' } : i
    ));
    
    try {
      const res = await fetch(`/api/github/user?token=${encodeURIComponent(token)}`);
      if (res.ok) {
        const user = await res.json();
        setIntegrations(prev => prev.map(i => 
          i.id === 'github' ? { 
            ...i, 
            status: 'connected', 
            user: { login: user.login, avatar_url: user.avatar_url },
            lastSync: new Date().toLocaleString()
          } : i
        ));
      } else {
        localStorage.removeItem('github_token');
        setIntegrations(prev => prev.map(i => 
          i.id === 'github' ? { ...i, status: 'disconnected', user: undefined } : i
        ));
      }
    } catch {
      setIntegrations(prev => prev.map(i => 
        i.id === 'github' ? { ...i, status: 'disconnected', user: undefined } : i
      ));
    }
  },
  
  checkGoogle: async () => {
    const token = localStorage.getItem('google_token');
    if (!token) {
      setIntegrations(prev => prev.map(i => 
        i.id === 'google' ? { ...i, status: 'disconnected', user: undefined } : i
      ));
      return;
    }
    
    setIntegrations(prev => prev.map(i => 
      i.id === 'google' ? { ...i, status: 'loading' } : i
    ));
    
    try {
      const res = await fetch(`/api/google/user?token=${encodeURIComponent(token)}`);
      if (res.ok) {
        const user = await res.json();
        setIntegrations(prev => prev.map(i => 
          i.id === 'google' ? { 
            ...i, 
            status: 'connected', 
            user: { login: user.email, avatar_url: user.picture || '' },
            lastSync: new Date().toLocaleString()
          } : i
        ));
      } else {
        localStorage.removeItem('google_token');
        setIntegrations(prev => prev.map(i => 
          i.id === 'google' ? { ...i, status: 'disconnected', user: undefined } : i
        ));
      }
    } catch {
      setIntegrations(prev => prev.map(i => 
        i.id === 'google' ? { ...i, status: 'disconnected', user: undefined } : i
      ));
    }
  },
  
  checkCloudflare: async () => {
    const token = localStorage.getItem('cloudflare_token');
    if (!token) {
      setIntegrations(prev => prev.map(i =>
        i.id === 'cloudflare' ? { ...i, status: 'disconnected', user: undefined } : i
      ));
      return;
    }

    setIntegrations(prev => prev.map(i =>
      i.id === 'cloudflare' ? { ...i, status: 'loading' } : i
    ));

    try {
      const res = await fetch(`/api/cloudflare/user?token=${encodeURIComponent(token)}`);
      if (res.ok) {
        const user = await res.json();
        setIntegrations(prev => prev.map(i =>
          i.id === 'cloudflare' ? {
            ...i,
            status: 'connected',
            user: { login: user.email || 'Cloudflare', avatar_url: '' },
            lastSync: new Date().toLocaleString()
          } : i
        ));
      } else {
        localStorage.removeItem('cloudflare_token');
        setIntegrations(prev => prev.map(i =>
          i.id === 'cloudflare' ? { ...i, status: 'disconnected', user: undefined } : i
        ));
      }
    } catch {
      setIntegrations(prev => prev.map(i =>
        i.id === 'cloudflare' ? { ...i, status: 'disconnected', user: undefined } : i
      ));
    }
  },

  checkLLM: () => {
    const hasProvider = llmRouter.getEnabledProviders().length > 0;
    setIntegrations(prev => prev.map(i =>
      i.id === 'llm' ? {
        ...i,
        status: hasProvider ? 'connected' : 'disconnected',
        user: hasProvider ? { login: 'Configured', avatar_url: '' } : undefined,
        lastSync: hasProvider ? new Date().toLocaleString() : undefined
      } : i
    ));
  },

  checkQwen: async () => {
    const tokenStr = localStorage.getItem('qwen_token');
    if (!tokenStr) {
      setIntegrations(prev => prev.map(i =>
        i.id === 'qwen' ? { ...i, status: 'disconnected', user: undefined } : i
      ));
      return;
    }

    try {
      const token: QwenToken = JSON.parse(tokenStr);
      
      // Check if token is expired
      if (token.expiry_date && Date.now() > token.expiry_date) {
        // Token expired, try to refresh
        const refreshToken = localStorage.getItem('qwen_refresh');
        if (refreshToken) {
          await integrationStore.refreshQwenToken(refreshToken);
          return;
        }
        // No refresh token, mark as disconnected
        setIntegrations(prev => prev.map(i =>
          i.id === 'qwen' ? { ...i, status: 'disconnected', user: undefined } : i
        ));
        return;
      }

      // Token is valid
      setIntegrations(prev => prev.map(i =>
        i.id === 'qwen' ? {
          ...i,
          status: 'connected',
          user: { login: 'Qwen', avatar_url: '' },
          lastSync: new Date().toLocaleString()
        } : i
      ));
    } catch {
      setIntegrations(prev => prev.map(i =>
        i.id === 'qwen' ? { ...i, status: 'disconnected', user: undefined } : i
      ));
    }
  },

  refreshQwenToken: async (refreshToken: string) => {
    try {
      const response = await fetch('https://portal.qwen.ai/oauth/token', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/x-www-form-urlencoded',
          'Accept': 'application/json'
        },
        body: new URLSearchParams({
          grant_type: 'refresh_token',
          client_id: import.meta.env.QWEN_CLIENT_ID || '',
          client_secret: import.meta.env.QWEN_CLIENT_SECRET || '',
          refresh_token: refreshToken,
        })
      });

      if (response.ok) {
        const tokenData = await response.json() as QwenToken;
        await integrationStore.connectQwen(tokenData);
      } else {
        // Refresh failed, clear tokens
        localStorage.removeItem('qwen_token');
        localStorage.removeItem('qwen_refresh');
      }
    } catch {
      // Refresh failed
      localStorage.removeItem('qwen_token');
      localStorage.removeItem('qwen_refresh');
    }
  },

  checkAll: async () => {
    if (isInitialized()) return;

    await Promise.all([
      integrationStore.checkGitHub(),
      integrationStore.checkGoogle(),
      integrationStore.checkCloudflare(),
      integrationStore.checkQwen(),
    ]);

    // Check LLM provider (synchronous)
    integrationStore.checkLLM();
    
    // Auto-configure Qwen provider if OAuth token exists but no provider
    const qwenTokenStr = localStorage.getItem('qwen_token');
    if (qwenTokenStr && llmRouter.getEnabledProviders().length === 0) {
      try {
        const qwenToken: QwenToken = JSON.parse(qwenTokenStr);
        if (qwenToken.access_token && Date.now() < qwenToken.expiry_date) {
          const qwenProvider = {
            id: 'qwen-oauth-' + Date.now(),
            name: 'Qwen Code',
            type: 'qwen' as const,
            baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
            apiKey: qwenToken.access_token,
            defaultModel: 'qwen-plus',
            availableModels: ['qwen-plus', 'qwen-turbo', 'qwen-max', 'qwen3.5-coder-plus'],
            enabled: true,
            priority: 1,
          };
          llmRouter.addProvider(qwenProvider);
          console.log('[Integrations] Auto-configured Qwen LLM provider');
        }
      } catch (e) {
        console.error('[Integrations] Failed to auto-configure Qwen:', e);
      }
    }

    setIsInitialized(true);
  },
  
  initFromUrl: () => {
    const params = new URLSearchParams(window.location.search);

    const githubToken = params.get('github_token');
    if (githubToken) {
      localStorage.setItem('github_token', githubToken);
      window.history.replaceState({}, '', window.location.pathname);
    }

    const googleToken = params.get('google_token');
    const googleRefresh = params.get('google_refresh');
    if (googleToken) {
      localStorage.setItem('google_token', googleToken);
      if (googleRefresh) localStorage.setItem('google_refresh', googleRefresh);
      window.history.replaceState({}, '', window.location.pathname);
    }

    // Handle Qwen OAuth token (JSON string)
    const qwenTokenStr = params.get('qwen_token');
    if (qwenTokenStr) {
      try {
        const qwenToken: QwenToken = JSON.parse(qwenTokenStr);
        localStorage.setItem('qwen_token', qwenTokenStr);
        if (qwenToken.refresh_token) {
          localStorage.setItem('qwen_refresh', qwenToken.refresh_token);
        }
        
        // Auto-configure LLM provider with Qwen OAuth
        const qwenProvider = {
          id: 'qwen-oauth-' + Date.now(),
          name: 'Qwen Code',
          type: 'qwen' as const,
          baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
          apiKey: qwenToken.access_token,
          defaultModel: 'qwen-plus',
          availableModels: ['qwen-plus', 'qwen-turbo', 'qwen-max', 'qwen3.5-coder-plus'],
          enabled: true,
          priority: 1,
        };
        llmRouter.addProvider(qwenProvider);
        
        console.log('[Qwen OAuth] LLM provider auto-configured');
      } catch (e) {
        console.error('[Qwen OAuth] Failed to parse token:', e);
      }
      window.history.replaceState({}, '', window.location.pathname);
    }

    const qwenSession = params.get('qwen_session');
    if (qwenSession) {
      window.history.replaceState({}, '', window.location.pathname);
    }

    const error = params.get('error');
    if (error) {
      window.history.replaceState({}, '', window.location.pathname);
      return error;
    }

    return null;
  },
};
