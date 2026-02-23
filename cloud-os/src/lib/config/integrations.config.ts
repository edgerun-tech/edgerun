import type { Component } from 'solid-js';
import {
  TbOutlineCode,
  TbOutlineMail,
  TbOutlinePackage,
  TbOutlineFileText,
  TbOutlineSun,
  TbOutlineBrandGithub,
  TbOutlineTerminal2,
  TbOutlinePhone,
  TbOutlineActivity,
  TbOutlineApps,
  TbOutlineGridDots,
  TbOutlineCloudBolt,
  TbOutlineDatabase,
  TbOutlineCloud,
} from 'solid-icons/tb';

export type AuthType = 'oauth' | 'token' | 'none';
export type WindowSize = 'small' | 'medium' | 'large' | 'fullscreen';

export interface IntegrationConfig {
  id: string;
  name: string;
  category: 'system' | 'cloud' | 'communication' | 'development';
  description: string;
  icon: string; // Icon identifier
  window: {
    title: string;
    defaultWidth: number;
    defaultHeight: number;
    minWidth?: number;
    minHeight?: number;
  };
  auth: {
    type: AuthType;
    scopes?: string[];
    tokenKey?: string;
  };
  dockItem?: boolean; // Show in dock
  integrationsPanel?: boolean; // Show in integrations panel
  enabled: boolean; // Can be disabled by user
}

// Icon registry - maps string identifiers to actual components
export const iconRegistry: Record<string, any> = {
  editor: TbOutlineCode,
  terminal: TbOutlineTerminal2,
  mail: TbOutlineMail,
  package: TbOutlinePackage,
  file: TbOutlineFileText,
  sun: TbOutlineSun,
  github: TbOutlineBrandGithub,
  phone: TbOutlinePhone,
  activity: TbOutlineActivity,
  apps: TbOutlineApps,
  grid: TbOutlineGridDots,
  cloudflare: TbOutlineCloudBolt,
  database: TbOutlineDatabase,
  google: TbOutlineMail,
  drive: TbOutlineDatabase,
  vercel: TbOutlineCloudBolt,
  cloud: TbOutlineCloud,
};

// Central configuration for all integrations
export const integrations: IntegrationConfig[] = [
  // System apps
  {
    id: 'editor',
    name: 'Editor',
    category: 'system',
    description: 'Code editor with syntax highlighting and file management',
    icon: 'editor',
    window: {
      title: 'Editor',
      defaultWidth: 800,
      defaultHeight: 600,
      minWidth: 400,
      minHeight: 300
    },
    auth: { type: 'none' },
    dockItem: true,
    integrationsPanel: false,
    enabled: true
  },
  {
    id: 'terminal',
    name: 'Terminal',
    category: 'system',
    description: 'Terminal emulator for command-line operations',
    icon: 'terminal',
    window: {
      title: 'Terminal',
      defaultWidth: 700,
      defaultHeight: 450,
      minWidth: 400,
      minHeight: 300
    },
    auth: { type: 'none' },
    dockItem: true,
    integrationsPanel: false,
    enabled: true
  },
  {
    id: 'files',
    name: 'Files',
    category: 'system',
    description: 'File manager with local, cloud, and device sources',
    icon: 'file',
    window: {
      title: 'Files',
      defaultWidth: 800,
      defaultHeight: 500,
      minWidth: 500,
      minHeight: 400
    },
    auth: { type: 'none' },
    dockItem: true,
    integrationsPanel: false,
    enabled: true
  },
  {
    id: 'settings',
    name: 'Settings',
    category: 'system',
    description: 'System settings and LLM provider configuration',
    icon: 'sun',
    window: {
      title: 'Settings',
      defaultWidth: 700,
      defaultHeight: 600
    },
    auth: { type: 'none' },
    dockItem: false,
    integrationsPanel: false,
    enabled: true
  },
  {
    id: 'integrations',
    name: 'Integrations',
    category: 'system',
    description: 'Manage connected services and cloud providers',
    icon: 'apps',
    window: {
      title: 'Integrations',
      defaultWidth: 700,
      defaultHeight: 600
    },
    auth: { type: 'none' },
    dockItem: false,
    integrationsPanel: false,
    enabled: true
  },
  {
    id: 'call',
    name: 'Call',
    category: 'system',
    description: 'Video calling with messaging',
    icon: 'phone',
    window: {
      title: 'Call',
      defaultWidth: 600,
      defaultHeight: 500
    },
    auth: { type: 'none' },
    dockItem: true,
    integrationsPanel: false,
    enabled: true
  },

  // Unified Cloud Panel
  {
    id: 'cloud',
    name: 'Cloud',
    category: 'cloud',
    description: 'Unified cloud resource management across all providers',
    icon: 'cloud',
    window: {
      title: 'Cloud',
      defaultWidth: 1000,
      defaultHeight: 700,
      minWidth: 600,
      minHeight: 400
    },
    auth: { type: 'none' },
    dockItem: true,
    integrationsPanel: false,
    enabled: true
  },

  // Cloud Integrations (individual panels hidden from dock)
  {
    id: 'github',
    name: 'GitHub',
    category: 'development',
    description: 'Browse repositories and edit code directly',
    icon: 'github',
    window: {
      title: 'GitHub',
      defaultWidth: 1000,
      defaultHeight: 700
    },
    auth: { 
      type: 'oauth',
      scopes: ['repo', 'user']
    },
    dockItem: false,
    integrationsPanel: true,
    enabled: true
  },
  {
    id: 'cloudflare',
    name: 'Cloudflare',
    category: 'cloud',
    description: 'Manage DNS, Tunnels, Pages, and Workers',
    icon: 'cloudflare',
    window: {
      title: 'Cloudflare',
      defaultWidth: 800,
      defaultHeight: 600
    },
    auth: { 
      type: 'token',
      tokenKey: 'cloudflare_token'
    },
    dockItem: false,
    integrationsPanel: true,
    enabled: true
  },
  {
    id: 'drive',
    name: 'Drive',
    category: 'cloud',
    description: 'Google Drive file browser and sync',
    icon: 'drive',
    window: {
      title: 'Drive',
      defaultWidth: 800,
      defaultHeight: 500
    },
    auth: { 
      type: 'oauth',
      scopes: ['https://www.googleapis.com/auth/drive.readonly']
    },
    dockItem: false,
    integrationsPanel: true,
    enabled: true
  },
  {
    id: 'calendar',
    name: 'Calendar',
    category: 'communication',
    description: 'Google Calendar integration',
    icon: 'sun',
    window: {
      title: 'Calendar',
      defaultWidth: 700,
      defaultHeight: 500
    },
    auth: { 
      type: 'oauth',
      scopes: ['https://www.googleapis.com/auth/calendar.readonly']
    },
    dockItem: false,
    integrationsPanel: true,
    enabled: true
  },
  {
    id: 'email',
    name: 'Email',
    category: 'communication',
    description: 'Email client with Gmail integration',
    icon: 'mail',
    window: {
      title: 'Email',
      defaultWidth: 900,
      defaultHeight: 600
    },
    auth: { 
      type: 'oauth',
      scopes: ['https://www.googleapis.com/auth/gmail.readonly']
    },
    dockItem: false,
    integrationsPanel: true,
    enabled: true
  },

  // Cloud integrations
  {
    id: 'vercel',
    name: 'Vercel',
    category: 'cloud',
    description: 'Manage Vercel deployments, projects, and domains',
    icon: 'vercel',
    window: {
      title: 'Vercel',
      defaultWidth: 900,
      defaultHeight: 700
    },
    auth: { 
      type: 'token',
      tokenKey: 'vercel_token'
    },
    dockItem: false,
    integrationsPanel: true,
    enabled: true
  },
  {
    id: 'hetzner',
    name: 'Hetzner',
    category: 'cloud',
    description: 'Hetzner Cloud - Manage servers, firewalls, and DNS',
    icon: 'cloudflare',
    window: {
      title: 'Hetzner',
      defaultWidth: 900,
      defaultHeight: 700
    },
    auth: { 
      type: 'token',
      tokenKey: 'hetzner_token'
    },
    dockItem: false,
    integrationsPanel: true,
    enabled: true
  }
];

// Helper functions
export const getEnabledIntegrations = () => integrations.filter(i => i.enabled);
export const getDockItems = () => integrations.filter(i => i.enabled && i.dockItem);
export const getIntegrationById = (id: string) => integrations.find(i => i.id === id);
export const getIntegrationsByCategory = (category: IntegrationConfig['category']) => 
  integrations.filter(i => i.category === category && i.enabled);
export const getIntegrationAuthConfig = (id: string) => {
  const integration = getIntegrationById(id);
  return integration?.auth;
};

// User preferences (can be persisted)
export interface UserIntegrationPreferences {
  enabled: Record<string, boolean>; // Override enabled state
  hidden: string[]; // Hidden from dock
}

export const defaultPreferences: UserIntegrationPreferences = {
  enabled: {},
  hidden: []
};
