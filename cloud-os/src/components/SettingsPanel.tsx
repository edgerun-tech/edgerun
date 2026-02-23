/**
 * Settings Panel Component
 * LLM provider configuration
 */

import { createSignal, Show, For, onMount } from 'solid-js';
import { TbOutlineBrain, TbOutlineTrash, TbOutlineCheck } from 'solid-icons/tb';
import { llmRouter } from '../lib/llm/router';
import type { LLMProvider, LLMProviderType } from '../lib/llm/types';

export default function SettingsPanel() {
  const [providers, setProviders] = createSignal<LLMProvider[]>([]);
  const [testing, setTesting] = createSignal<string | null>(null);
  const [testResult, setTestResult] = createSignal<{ id: string; success: boolean; message: string } | null>(null);

  const [newProvider, setNewProvider] = createSignal<Partial<LLMProvider>>({
    name: '',
    type: 'openai',
    baseUrl: 'https://api.openai.com/v1',
    defaultModel: 'gpt-4o',
    availableModels: [],
    enabled: true,
    priority: 10,
  });

  onMount(async () => {
    const existingProviders = llmRouter.getProviders();
    setProviders(existingProviders);
  });

  const providerTypes: { value: LLMProviderType; label: string }[] = [
    { value: 'openai', label: 'OpenAI' },
    { value: 'anthropic', label: 'Anthropic' },
    { value: 'qwen', label: 'Qwen Code' },
    { value: 'ollama', label: 'Ollama (Local)' },
    { value: 'custom', label: 'Custom Endpoint' },
  ];

  const addProvider = () => {
    const provider: LLMProvider = {
      id: `provider-${Date.now()}`,
      name: newProvider().name || 'New Provider',
      type: newProvider().type || 'openai',
      baseUrl: newProvider().baseUrl || '',
      defaultModel: newProvider().defaultModel || 'gpt-4o',
      availableModels: newProvider().availableModels || [],
      enabled: true,
      priority: newProvider().priority || 10,
    };

    llmRouter.addProvider(provider);
    setProviders([...providers(), provider]);
    setNewProvider({
      name: '',
      type: 'openai',
      baseUrl: 'https://api.openai.com/v1',
      defaultModel: 'gpt-4o',
      availableModels: [],
      enabled: true,
      priority: 10,
    });
  };

  const removeProvider = (id: string) => {
    llmRouter.removeProvider(id);
    setProviders(providers().filter(p => p.id !== id));
  };

  const testProvider = async (provider: LLMProvider) => {
    setTesting(provider.id);
    setTestResult(null);

    try {
      const response = await fetch(`${provider.baseUrl}/models`, {
        headers: provider.apiKey ? { 'Authorization': `Bearer ${provider.apiKey}` } : {},
      });

      const success = response.ok;
      const message = success 
        ? 'Connection successful!'
        : `Failed: ${response.statusText}`;

      setTestResult({ id: provider.id, success, message });
    } catch (error) {
      setTestResult({ 
        id: provider.id, 
        success: false, 
        message: `Error: ${error instanceof Error ? error.message : 'Unknown'}` 
      });
    }

    setTesting(null);
  };

  return (
    <div class="h-full flex flex-col bg-[#1a1a1a] text-neutral-200 p-4 overflow-auto">
      <div class="mb-6">
        <h2 class="text-lg font-semibold text-white flex items-center gap-2">
          <TbOutlineBrain size={20} />
          LLM Providers
        </h2>
        <p class="text-sm text-neutral-500 mt-1">
          Configure AI providers for the Intent Bar
        </p>
      </div>

      <div class="space-y-4">
        <For each={providers()}>
          {(provider) => (
            <div class="p-3 bg-neutral-800 rounded-lg">
              <div class="flex items-center justify-between mb-2">
                <div class="flex items-center gap-2">
                  <span class="text-white font-medium">{provider.name}</span>
                  <span class="text-neutral-500 text-sm">({provider.type})</span>
                </div>
                <div class="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={() => testProvider(provider)}
                    disabled={testing() === provider.id}
                    class="px-2 py-1 text-xs bg-neutral-700 hover:bg-neutral-600 rounded text-neutral-300 disabled:opacity-50"
                  >
                    {testing() === provider.id ? 'Testing...' : 'Test'}
                  </button>
                  <button
                    type="button"
                    onClick={() => removeProvider(provider.id)}
                    class="p-1 text-red-400 hover:bg-neutral-700 rounded"
                  >
                    <TbOutlineTrash size={14} />
                  </button>
                </div>
              </div>
              <Show when={testResult()?.id === provider.id}>
                <div class={`text-xs mb-2 ${testResult()!.success ? 'text-green-400' : 'text-red-400'}`}>
                  {testResult()!.message}
                </div>
              </Show>
              <div class="text-xs text-neutral-500">
                {provider.baseUrl} • {provider.defaultModel}
              </div>
            </div>
          )}
        </For>
      </div>

      <div class="mt-6 p-4 bg-neutral-800/50 rounded-lg border border-neutral-700">
        <h4 class="text-sm font-medium text-white mb-3">Add Provider</h4>
        
        <div class="space-y-3">
          <div>
            <label class="block text-xs text-neutral-400 mb-1">Name</label>
            <input
              type="text"
              value={newProvider().name || ''}
              onInput={(e) => setNewProvider(p => ({ ...p, name: e.currentTarget.value }))}
              placeholder="My OpenAI"
              class="w-full px-3 py-2 bg-neutral-900 border border-neutral-700 rounded text-sm text-white"
            />
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-xs text-neutral-400 mb-1">Type</label>
              <select
                value={newProvider().type}
                onChange={(e) => {
                  const type = e.currentTarget.value as LLMProviderType;
                  let baseUrl = '';
                  let defaultModel = '';

                  if (type === 'openai') {
                    baseUrl = 'https://api.openai.com/v1';
                    defaultModel = 'gpt-4o';
                  } else if (type === 'anthropic') {
                    baseUrl = 'https://api.anthropic.com';
                    defaultModel = 'claude-3-5-sonnet-20241022';
                  } else if (type === 'qwen') {
                    baseUrl = 'https://dashscope.aliyuncs.com/compatible-mode/v1';
                    defaultModel = 'qwen-plus';
                  } else if (type === 'ollama') {
                    baseUrl = 'http://localhost:11434';
                    defaultModel = 'llama3.2';
                  }

                  setNewProvider(p => ({ ...p, type, baseUrl, defaultModel }));
                }}
                class="w-full px-3 py-2 bg-neutral-900 border border-neutral-700 rounded text-sm text-white"
              >
                <For each={providerTypes}>
                  {(pt) => <option value={pt.value}>{pt.label}</option>}
                </For>
              </select>
            </div>

            <div>
              <label class="block text-xs text-neutral-400 mb-1">Priority</label>
              <input
                type="number"
                value={newProvider().priority || 10}
                onInput={(e) => setNewProvider(p => ({ ...p, priority: parseInt(e.currentTarget.value) }))}
                class="w-full px-3 py-2 bg-neutral-900 border border-neutral-700 rounded text-sm text-white"
              />
            </div>
          </div>

          <div>
            <label class="block text-xs text-neutral-400 mb-1">Base URL</label>
            <input
              type="text"
              value={newProvider().baseUrl || ''}
              onInput={(e) => setNewProvider(p => ({ ...p, baseUrl: e.currentTarget.value }))}
              placeholder="https://api.openai.com/v1"
              class="w-full px-3 py-2 bg-neutral-900 border border-neutral-700 rounded text-sm text-white"
            />
          </div>

          <div>
            <label class="block text-xs text-neutral-400 mb-1">Default Model</label>
            <input
              type="text"
              value={newProvider().defaultModel || ''}
              onInput={(e) => setNewProvider(p => ({ ...p, defaultModel: e.currentTarget.value }))}
              placeholder="gpt-4o"
              class="w-full px-3 py-2 bg-neutral-900 border border-neutral-700 rounded text-sm text-white"
            />
          </div>

          <Show when={newProvider().type !== 'ollama'}>
            <div>
              <label class="block text-xs text-neutral-400 mb-1">API Key</label>
              <input
                type="password"
                value={newProvider().apiKey || ''}
                onInput={(e) => setNewProvider(p => ({ ...p, apiKey: e.currentTarget.value }))}
                placeholder="sk-..."
                class="w-full px-3 py-2 bg-neutral-900 border border-neutral-700 rounded text-sm text-white"
              />
            </div>
          </Show>

          <button
            type="button"
            onClick={addProvider}
            disabled={!newProvider().name || !newProvider().baseUrl}
            class="w-full py-2 bg-blue-600 hover:bg-blue-500 rounded text-sm font-medium text-white disabled:opacity-50 flex items-center justify-center gap-2"
          >
            <TbOutlineCheck size={16} />
            Add Provider
          </button>
        </div>
      </div>
    </div>
  );
}
