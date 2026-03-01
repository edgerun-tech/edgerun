/**
 * @typedef {"openai" | "anthropic" | "ollama" | "custom"} LLMProviderType
 */

/**
 * @typedef {object} LLMProvider
 * @property {string} id
 * @property {string} name
 * @property {LLMProviderType} type
 * @property {string} baseUrl
 * @property {string=} apiKey
 * @property {string} defaultModel
 * @property {string[]} availableModels
 * @property {boolean} enabled
 * @property {number} priority
 */

export {};
