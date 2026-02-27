let providers = [];
const defaultProviders = [];
const llmRouter = {
  addProvider(provider) {
    const existing = providers.find((p) => p.id === provider.id);
    if (existing) {
      providers = providers.map((p) => p.id === provider.id ? provider : p);
      return;
    }
    providers = [...providers, provider].sort((a, b) => a.priority - b.priority);
  },
  removeProvider(id) {
    providers = providers.filter((p) => p.id !== id);
  },
  getProviders() {
    return [...providers];
  },
  getEnabledProviders() {
    return providers.filter((p) => p.enabled);
  }
};
export {
  defaultProviders,
  llmRouter
};
