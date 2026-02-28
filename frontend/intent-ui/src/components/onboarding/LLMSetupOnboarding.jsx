import { createSignal, Show, For } from "solid-js";
import { icons } from "../../lib/icons";
import { llmRouter } from "../../lib/llm/router";
const STORAGE_KEY = "cloudos-llm-setup-dismissed";
const providerPresets = [
  {
    id: "openai",
    name: "OpenAI",
    icon: "sparkles",
    description: "GPT-4o and other models",
    baseUrl: "https://api.openai.com/v1",
    defaultModel: "gpt-4o",
    models: ["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "o1-preview"],
    requiresApiKey: true,
    color: "text-green-400",
    bg: "bg-green-500/10"
  },
  {
    id: "anthropic",
    name: "Anthropic",
    icon: "brain",
    description: "Claude models",
    baseUrl: "https://api.anthropic.com",
    defaultModel: "claude-3-5-sonnet-20241022",
    models: ["claude-3-5-sonnet-20241022", "claude-3-opus-20240229", "claude-3-haiku-20240307"],
    requiresApiKey: true,
    color: "text-orange-400",
    bg: "bg-orange-500/10"
  },
  {
    id: "qwen",
    name: "Qwen Code",
    icon: "code",
    description: "Requires DashScope API key (not OAuth)",
    baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    defaultModel: "qwen-plus",
    models: ["qwen-plus", "qwen-turbo", "qwen-max"],
    requiresApiKey: true,
    requiresOAuth: false,
    color: "text-blue-400",
    bg: "bg-blue-500/10"
  },
  {
    id: "ollama",
    name: "Ollama (Local)",
    icon: "server",
    description: "Run models locally",
    baseUrl: "http://localhost:11434",
    defaultModel: "llama3.2",
    models: ["llama3.2", "mistral", "codellama", "phi3"],
    requiresApiKey: false,
    color: "text-blue-400",
    bg: "bg-blue-500/10"
  },
  {
    id: "custom",
    name: "Custom",
    icon: "lock",
    description: "Any OpenAI-compatible API",
    baseUrl: "",
    defaultModel: "",
    models: [],
    requiresApiKey: true,
    color: "text-purple-400",
    bg: "bg-purple-500/10"
  }
];
const DEFAULT_PROVIDER_ICON_COLOR = "text-neutral-300";
const DEFAULT_PROVIDER_ICON_BG = "bg-neutral-700";
function LLMSetupOnboarding() {
  const [dismissed, setDismissed] = createSignal(false);
  const [selectedProvider, setSelectedProvider] = createSignal(null);
  const [apiKey, setApiKey] = createSignal("");
  const [customName, setCustomName] = createSignal("");
  const [customBaseUrl, setCustomBaseUrl] = createSignal("");
  const [customModel, setCustomModel] = createSignal("");
  const [testing, setTesting] = createSignal(false);
  const [testSuccess, setTestSuccess] = createSignal(null);
  const [configured, setConfigured] = createSignal(false);
  const [error, setError] = createSignal(null);
  const isDismissed = () => {
    if (typeof window === "undefined") return false;
    return localStorage.getItem(STORAGE_KEY) === "true";
  };
  const hasProvider = () => {
    return llmRouter.getEnabledProviders().length > 0;
  };
  if (dismissed() || isDismissed() || hasProvider()) return null;
  const handleSelectProvider = (preset) => {
    setSelectedProvider(preset);
    setTestSuccess(null);
    setTesting(false);
    if (preset.id === "openai") {
      setCustomName("OpenAI");
      setCustomBaseUrl("https://api.openai.com/v1");
      setCustomModel("gpt-4o");
    } else if (preset.id === "anthropic") {
      setCustomName("Anthropic");
      setCustomBaseUrl("https://api.anthropic.com");
      setCustomModel("claude-3-5-sonnet-20241022");
    } else if (preset.id === "qwen") {
      setCustomName("Qwen Code");
      setCustomBaseUrl("https://dashscope.aliyuncs.com/compatible-mode/v1");
      setCustomModel("qwen-plus");
    } else if (preset.id === "ollama") {
      setCustomName("Ollama Local");
      setCustomBaseUrl("http://localhost:11434");
      setCustomModel("llama3.2");
    } else {
      setCustomName("");
      setCustomBaseUrl("");
      setCustomModel("");
    }
  };
  const initiateOAuth = async () => {
    setTesting(true);
    setError(null);
    try {
      const deviceResponse = await fetch("/api/qwen", {
        method: "GET",
        redirect: "manual"
      });
      if (deviceResponse.type === "opaqueredirect") {
        const authWindow = window.open("/api/qwen", "qwen-auth", "width=500,height=600");
        setTestSuccess("Please complete authentication in the popup window...");
        const pollInterval = setInterval(async () => {
          try {
            const pollResponse = await fetch("/api/qwen/poll", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ check_token: true })
            });
            const data = await pollResponse.json();
            if (pollResponse.ok && data.access_token) {
              clearInterval(pollInterval);
              if (authWindow) authWindow.close();
              localStorage.setItem("qwen_token", JSON.stringify(data));
              const provider = {
                id: `qwen-oauth-${Date.now()}`,
                name: "Qwen Code",
                type: "qwen",
                baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
                apiKey: data.access_token,
                defaultModel: "qwen-plus",
                availableModels: ["qwen-plus", "qwen-turbo", "qwen-max"],
                enabled: true,
                priority: 5
              };
              llmRouter.addProvider(provider);
              setConfigured(true);
              setTestSuccess("Qwen connected successfully!");
            }
          } catch (e) {
            console.log("[Qwen] Polling...", e);
          }
        }, 3e3);
        setTimeout(() => clearInterval(pollInterval), 6e5);
      }
      setTesting(false);
    } catch (error2) {
      console.error("[LLMSetup] Device flow error:", error2);
      setError("Failed to start device flow");
      setTesting(false);
    }
  };
  const testConnection = async () => {
    if (!selectedProvider()) return;
    setTesting(true);
    setTestSuccess(null);
    const provider = {
      id: `test-${Date.now()}`,
      name: customName() || selectedProvider().name,
      type: selectedProvider().id,
      baseUrl: customBaseUrl() || selectedProvider().baseUrl,
      apiKey: apiKey(),
      defaultModel: customModel() || selectedProvider().defaultModel,
      availableModels: selectedProvider().models,
      enabled: true,
      priority: 10
    };
    try {
      let response;
      if (provider.type === "ollama") {
        response = await fetch(`${provider.baseUrl}/api/tags`);
      } else {
        response = await fetch(`${provider.baseUrl}/models`, {
          headers: provider.apiKey ? { "Authorization": `Bearer ${provider.apiKey}` } : {}
        });
      }
      const success = response.ok;
      setTestSuccess(success ? "Connection successful!" : null);
      if (success) {
        setConfigured(true);
      }
    } catch {
      setTestSuccess("Connection failed");
    }
    setTesting(false);
  };
  const saveProvider = () => {
    if (!selectedProvider()) return;
    const provider = {
      id: `provider-${Date.now()}`,
      name: customName() || selectedProvider().name,
      type: selectedProvider().id,
      baseUrl: customBaseUrl() || selectedProvider().baseUrl,
      apiKey: apiKey(),
      defaultModel: customModel() || selectedProvider().defaultModel,
      availableModels: selectedProvider().models,
      enabled: true,
      priority: selectedProvider().id === "ollama" ? 1 : 10
    };
    llmRouter.addProvider(provider);
    setConfigured(true);
  };
  const dismiss = () => {
    setDismissed(true);
    if (typeof window !== "undefined") {
      localStorage.setItem(STORAGE_KEY, "true");
    }
  };
  const skipForNow = () => {
    dismiss();
  };
  return <div class="fixed inset-0 z-[10000] flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div class="bg-[#1a1a1a] rounded-2xl border border-neutral-800 shadow-2xl w-full max-w-2xl mx-4 overflow-hidden">
        {
    /* Header */
  }
        <div class="p-6 border-b border-neutral-800 flex items-center justify-between">
          <div class="flex items-center gap-3">
            <div class="p-2.5 bg-purple-500/20 rounded-xl">
              <icons.sparkles size={22} class="text-purple-400" />
            </div>
            <div>
              <h2 class="text-xl font-semibold text-white">Setup AI Assistant</h2>
              <p class="text-sm text-neutral-500">Configure an LLM provider to use AI commands</p>
            </div>
          </div>
          <button
    type="button"
    onClick={dismiss}
    class="p-2 text-neutral-500 hover:text-white hover:bg-neutral-800 rounded-lg transition-colors"
  >
            <icons.close size={18} />
          </button>
        </div>

        <div class="p-6">
          <Show when={!configured()} fallback={<div class="text-center py-8">
              <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-green-500/20 flex items-center justify-center">
                <icons.check size={32} class="text-green-400" />
              </div>
              <h3 class="text-lg font-semibold text-white mb-2">Provider Configured!</h3>
              <p class="text-neutral-500 mb-6">You can now use AI commands in the Intent Bar.</p>
              <button
    type="button"
    onClick={dismiss}
    class="px-6 py-2.5 bg-white text-black font-medium rounded-lg hover:bg-neutral-200 transition-colors"
  >
                Get Started
              </button>
            </div>}>
            {
    /* Provider Selection */
  }
            <Show when={!selectedProvider()} fallback={
    /* Configuration Form */
    <div class="space-y-4">
                <button
      type="button"
      onClick={() => setSelectedProvider(null)}
      class="text-sm text-neutral-400 hover:text-white flex items-center gap-1 mb-4"
    >
                  ← Back to providers
                </button>

                <div class="p-4 bg-neutral-800/50 rounded-xl border border-neutral-700">
                  <div class="flex items-center gap-3 mb-4">
                    {(() => {
      const provider = selectedProvider();
      if (!provider) return null;
      const iconKey = provider.icon;
      const Icon = icons[iconKey];
      if (!Icon) return null;
      return <div class={`p-2 rounded-lg ${provider.bg || DEFAULT_PROVIDER_ICON_BG}`}>
                          <Icon size={20} class={provider.color || DEFAULT_PROVIDER_ICON_COLOR} />
                        </div>;
    })()}
                    <div>
                      <h3 class="font-medium text-white">{customName() || selectedProvider()?.name || "Provider"}</h3>
                      <p class="text-xs text-neutral-500">{selectedProvider()?.description || "Configure provider settings."}</p>
                    </div>
                  </div>

                  <div class="space-y-3">
                    <Show when={!selectedProvider()?.requiresOAuth} fallback={
      /* OAuth Flow for Qwen */
      <div class="text-center py-6">
                        <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-blue-500/20 flex items-center justify-center">
                          <icons.shield size={32} class="text-blue-400" />
                        </div>
                        <h4 class="text-white font-medium mb-2">Connect with Qwen OAuth</h4>
                        <p class="text-sm text-neutral-500 mb-4">
                          You will be redirected to Alibaba Cloud to authenticate securely.
                        </p>
                        <button
        type="button"
        onClick={initiateOAuth}
        class="w-full py-2.5 bg-blue-600 hover:bg-blue-500 rounded-lg text-sm font-medium text-white transition-colors flex items-center justify-center gap-2"
      >
                          <icons.shield size={16} />
                          Connect with Qwen
                        </button>
                      </div>
    }>
                    <div>
                      <label class="block text-xs text-neutral-400 mb-1">Provider Name</label>
                      <input
      type="text"
      value={customName()}
      onInput={(e) => setCustomName(e.currentTarget.value)}
      class="w-full px-3 py-2 bg-neutral-900 border border-neutral-700 rounded-lg text-sm text-white focus:outline-none focus:border-purple-500"
    />
                    </div>

                    <div>
                      <label class="block text-xs text-neutral-400 mb-1">Base URL</label>
                      <input
      type="text"
      value={customBaseUrl()}
      onInput={(e) => setCustomBaseUrl(e.currentTarget.value)}
      class="w-full px-3 py-2 bg-neutral-900 border border-neutral-700 rounded-lg text-sm text-white focus:outline-none focus:border-purple-500"
    />
                    </div>

                    <div>
                      <label class="block text-xs text-neutral-400 mb-1">Default Model</label>
                      <input
      type="text"
      value={customModel()}
      onInput={(e) => setCustomModel(e.currentTarget.value)}
      placeholder="e.g., gpt-4o"
      class="w-full px-3 py-2 bg-neutral-900 border border-neutral-700 rounded-lg text-sm text-white focus:outline-none focus:border-purple-500"
    />
                      <Show when={(selectedProvider()?.models || []).length > 0}>
                        <div class="flex flex-wrap gap-1.5 mt-2">
                          <For each={selectedProvider()?.models || []}>
                            {(model) => <button
      type="button"
      onClick={() => setCustomModel(model)}
      class={`px-2 py-1 text-xs rounded transition-colors ${customModel() === model ? "bg-purple-500 text-white" : "bg-neutral-800 text-neutral-400 hover:text-white"}`}
    >
                                {model}
                              </button>}
                          </For>
                        </div>
                      </Show>
                    </div>

                    <Show when={selectedProvider()?.requiresApiKey}>
                      <div>
                        <label class="block text-xs text-neutral-400 mb-1">API Key</label>
                        <input
      type="password"
      value={apiKey()}
      onInput={(e) => setApiKey(e.currentTarget.value)}
      placeholder="sk-..."
      class="w-full px-3 py-2 bg-neutral-900 border border-neutral-700 rounded-lg text-sm text-white focus:outline-none focus:border-purple-500"
    />
                      </div>
                    </Show>

                    <Show when={testSuccess() !== null}>
                      <div class={`text-sm ${testSuccess() ? "text-green-400" : "text-red-400"}`}>
                        {testSuccess() ? "\u2713 Connection successful!" : "\u2717 Connection failed. Please check your settings."}
                      </div>
                    </Show>

                    <div class="flex gap-2 pt-2">
                      <button
      type="button"
      onClick={testConnection}
      disabled={testing()}
      class="flex-1 py-2.5 bg-neutral-700 hover:bg-neutral-600 rounded-lg text-sm font-medium text-white transition-colors disabled:opacity-50"
    >
                        {testing() ? "Testing..." : "Test Connection"}
                      </button>
                      <button
      type="button"
      onClick={saveProvider}
      class="flex-1 py-2.5 bg-purple-600 hover:bg-purple-500 rounded-lg text-sm font-medium text-white transition-colors flex items-center justify-center gap-2"
    >
                        <icons.check size={16} />
                        Save Provider
                      </button>
                    </div>
                    </Show>
                  </div>
                </div>
              </div>
  }>
              {
    /* Provider Grid */
  }
              <div class="mb-6">
                <p class="text-sm text-neutral-400 mb-4">
                  Choose an AI provider to enable natural language commands. Your API key is stored locally.
                </p>
                <div class="grid grid-cols-2 gap-3">
                  <For each={providerPresets}>
                    {(preset) => <button
    type="button"
    onClick={() => handleSelectProvider(preset)}
    class="p-4 rounded-xl border border-neutral-800 hover:border-neutral-600 hover:bg-neutral-800/50 transition-all text-left"
  >
                        <div class="flex items-start gap-3">
                          <div class={`p-2 rounded-lg ${preset.bg}`}>
                            {(() => {
    const Icon = icons[preset.icon];
    if (!Icon) return null;
    return <Icon size={20} class={preset.color || DEFAULT_PROVIDER_ICON_COLOR} />;
  })()}
                          </div>
                          <div class="flex-1 min-w-0">
                            <h3 class="font-medium text-white text-sm">{preset.name}</h3>
                            <p class="text-xs text-neutral-500 mt-0.5">{preset.description}</p>
                            <Show when={preset.requiresApiKey}>
                              <span class="inline-flex items-center gap-1 mt-2 text-xs text-neutral-500">
                                <icons.lock size={10} />
                                API key required
                              </span>
                            </Show>
                            <Show when={preset.requiresOAuth}>
                              <span class="inline-flex items-center gap-1 mt-2 text-xs text-blue-400">
                                <icons.shield size={10} />
                                OAuth 2.0
                              </span>
                            </Show>
                            <Show when={!preset.requiresApiKey && !preset.requiresOAuth}>
                              <span class="inline-flex items-center gap-1 mt-2 text-xs text-green-400">
                                <icons.sparkles size={10} />
                                No API key needed
                              </span>
                            </Show>
                          </div>
                          <icons.chevronRight size={16} class="text-neutral-600" />
                        </div>
                      </button>}
                  </For>
                </div>
              </div>

              {
    /* Quick Start Info */
  }
              <div class="p-4 bg-neutral-800/30 rounded-xl border border-neutral-800">
                <h4 class="text-sm font-medium text-white mb-2">Why setup an LLM?</h4>
                <ul class="space-y-1.5 text-sm text-neutral-400">
                  <li class="flex items-start gap-2">
                    <icons.chevronRight size={14} class="text-purple-400 mt-0.5" />
                    Use natural language to control your cloud
                  </li>
                  <li class="flex items-start gap-2">
                    <icons.chevronRight size={14} class="text-purple-400 mt-0.5" />
                    Automate complex tasks with AI assistance
                  </li>
                  <li class="flex items-start gap-2">
                    <icons.chevronRight size={14} class="text-purple-400 mt-0.5" />
                    Get intelligent suggestions and insights
                  </li>
                </ul>
              </div>
            </Show>
          </Show>
        </div>

        {
    /* Footer */
  }
        <Show when={!configured()}>
          <div class="p-4 border-t border-neutral-800 flex items-center justify-between">
            <button
    type="button"
    onClick={skipForNow}
    class="text-sm text-neutral-500 hover:text-white transition-colors"
  >
              Skip for now
            </button>
            <div class="flex items-center gap-2 text-xs text-neutral-500">
              <icons.lock size={12} />
              <span>API keys stored locally</span>
            </div>
          </div>
        </Show>
      </div>
    </div>;
}
export {
  LLMSetupOnboarding as default
};
