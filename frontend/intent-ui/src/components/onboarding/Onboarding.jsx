import { createSignal, Show, onMount } from "solid-js";
import { icons } from "../../lib/icons";
import { integrationStore } from "../../stores/integrations";
import { llmRouter } from "../../lib/llm/router";
import { Kbd, KbdGroup } from "../../registry/ui/kbd";
import { openWorkflowIntegrations } from "../../stores/workflow-ui";
import { openWindow } from "../../stores/windows";
import LLMSetupOnboarding from "./LLMSetupOnboarding";
const STORAGE_KEY = "cloudos-onboarding-dismissed";
function Onboarding() {
  const defaultFeatureColor = "text-neutral-300";
  const defaultFeatureBg = "bg-neutral-700";
  const [dismissed, setDismissed] = createSignal(false);
  const [showFeature, setShowFeature] = createSignal(null);
  const [showLLMSetup, setShowLLMSetup] = createSignal(false);
  onMount(() => {
    if (typeof window === "undefined") return;
    const wasDismissed = localStorage.getItem(STORAGE_KEY);
    if (wasDismissed) {
      setDismissed(true);
    }
    integrationStore.checkAll();
    const hasProvider = llmRouter.getEnabledProviders().length > 0;
    const llmDismissed = localStorage.getItem("cloudos-llm-setup-dismissed") === "true";
    if (!hasProvider && !llmDismissed) {
      setTimeout(() => setShowLLMSetup(true), 500);
    }
  });
  const dismiss = () => {
    setDismissed(true);
    if (typeof window !== "undefined") {
      localStorage.setItem(STORAGE_KEY, "true");
    }
  };
  const openIntegrations = () => {
    openWorkflowIntegrations("github");
  };
  if (dismissed()) return null;
  const githubConnected = () => integrationStore.isConnected("github");
  const cloudflareConnected = () => integrationStore.isConnected("cloudflare");
  const hasLLMProvider = () => llmRouter.getEnabledProviders().length > 0;
  const features = [
    {
      id: "llm",
      icon: "sparkles",
      title: "AI Provider",
      description: "Setup an LLM provider for AI commands",
      connected: hasLLMProvider(),
      action: () => setShowLLMSetup(true),
      color: "text-purple-400",
      bg: "bg-purple-500/10"
    },
    {
      id: "intent",
      icon: "sparkles",
      title: "AI Commands",
      description: "Use the command shortcut below to open the bar and type what you want to do.",
      examples: ['"restart my server"', '"list my repos"', '"deploy to production"'],
      color: "text-yellow-400",
      bg: "bg-yellow-500/10"
    },
    {
      id: "cloud",
      icon: "cloud",
      title: "Cloudflare",
      description: "Manage DNS, Workers, Tunnels, and Pages from any device.",
      connected: cloudflareConnected(),
      color: "text-orange-400",
      bg: "bg-orange-500/10"
    },
    {
      id: "github",
      icon: "github",
      title: "GitHub",
      description: "Browse repos, edit code, and manage pull requests.",
      connected: githubConnected(),
      color: "text-white",
      bg: "bg-neutral-700"
    },
    {
      id: "terminal",
      icon: "terminal",
      title: "Terminal",
      description: "Full terminal emulator with persistent sessions.",
      color: "text-green-400",
      bg: "bg-green-500/10"
    }
  ];
  return <div class="fixed bottom-20 left-4 z-[9999] w-80">
      <div class="bg-[#1a1a1a] rounded-2xl border border-neutral-800 shadow-2xl overflow-hidden">
        {
    /* Header */
  }
        <div class="p-4 border-b border-neutral-800 flex items-center justify-between">
          <div class="flex items-center gap-2">
            <div class="p-2 bg-blue-500/20 rounded-lg">
              <icons.cloud size={18} class="text-blue-400" />
            </div>
            <div>
              <h3 class="font-semibold text-white text-sm">Welcome to CloudOS</h3>
              <p class="text-xs text-neutral-500">Your cloud control center</p>
            </div>
          </div>
          <button
    type="button"
    onClick={dismiss}
    class="p-1.5 text-neutral-500 hover:text-white hover:bg-neutral-800 rounded-lg transition-colors"
  >
            <icons.close size={16} />
          </button>
        </div>

        {
    /* Features */
  }
        <div class="p-3 space-y-2">
          {features.map((feature) => <button
    type="button"
    class={`w-full text-left p-3 rounded-xl border transition-all ${showFeature() === feature.id ? "border-neutral-600 bg-neutral-800" : "border-transparent hover:border-neutral-700 hover:bg-neutral-800/50"}`}
    onClick={(e) => {
      e.stopPropagation();
      if (feature.action) {
        feature.action();
      } else {
        setShowFeature(showFeature() === feature.id ? null : feature.id);
      }
    }}
  >
              <div class="flex items-start gap-3">
                <div class={`p-2 rounded-lg ${feature.bg || defaultFeatureBg}`}>
                  {(() => {
    const Icon = icons[feature.icon];
    if (!Icon) return null;
    return <Icon size={18} class={feature.color || defaultFeatureColor} />;
  })()}
                </div>
                <div class="flex-1 min-w-0">
                  <div class="flex items-center justify-between">
                    <h4 class="font-medium text-white text-sm">{feature.title}</h4>
                    <Show when={feature.connected !== void 0}>
                      <Show when={feature.connected} fallback={<icons.chevronRight size={14} class="text-neutral-600" />}>
                        {(() => {
    const CheckIcon = icons.check;
    return CheckIcon ? <CheckIcon size={14} class="text-green-400" /> : null;
  })()}
                      </Show>
                    </Show>
                  </div>
                  <p class="text-xs text-neutral-500 mt-0.5 line-clamp-2">{feature.description}</p>
                </div>
              </div>

              {
    /* Expanded content */
  }
              <Show when={showFeature() === feature.id}>
                <div class="mt-3 pt-3 border-t border-neutral-700">
                  <Show when={feature.examples}>
                    <p class="text-xs text-neutral-400 mb-2">Try saying:</p>
                    <div class="space-y-1">
                      {feature.examples.map((ex) => <div class="text-xs text-neutral-300 bg-neutral-900 px-2 py-1.5 rounded font-mono">
                          {ex}
                        </div>)}
                    </div>
                  </Show>

                  <Show when={feature.id === "github" && !feature.connected}>
                    <button
    type="button"
    onClick={(e) => {
      e.stopPropagation();
      openIntegrations();
    }}
    class="mt-3 w-full py-2 bg-white text-black text-sm font-medium rounded-lg hover:bg-neutral-200 transition-colors"
  >
                      Connect GitHub
                    </button>
                  </Show>

                  <Show when={feature.id === "cloud" && !feature.connected}>
                    <button
    type="button"
    onClick={(e) => {
      e.stopPropagation();
      openIntegrations();
    }}
    class="mt-3 w-full py-2 bg-orange-600 text-white text-sm font-medium rounded-lg hover:bg-orange-500 transition-colors"
  >
                      Connect Cloudflare
                    </button>
                  </Show>
                </div>
              </Show>
            </button>)}
        </div>

        {
    /* Footer */
  }
        <div class="p-3 border-t border-neutral-800">
          <div class="flex items-center justify-between text-xs text-neutral-500">
            <span class="inline-flex items-center gap-2">
              <span>Press</span>
              <KbdGroup>
                <Kbd>Ctrl</Kbd>
                <span>+</span>
                <Kbd>Space</Kbd>
              </KbdGroup>
              <span>for commands</span>
            </span>
            <Show when={!githubConnected() || !cloudflareConnected()}>
              <button
    type="button"
    onClick={openIntegrations}
    class="text-blue-400 hover:text-blue-300 transition-colors flex items-center gap-1"
  >
                Setup integrations <icons.chevronRight size={12} />
              </button>
            </Show>
            <button
    type="button"
    onClick={() => openWindow("guide")}
    class="text-neutral-400 hover:text-neutral-200 transition-colors flex items-center gap-1"
  >
              Open guide <icons.chevronRight size={12} />
            </button>
          </div>
        </div>
      </div>

      {
    /* LLM Setup Modal */
  }
      <Show when={showLLMSetup()}>
        <LLMSetupOnboarding />
      </Show>
    </div>;
}
export {
  Onboarding as default
};
