/**
 * @typedef {"preview" | "json-tree" | "table" | "log-viewer" | "file-grid" | "code-diff" | "timeline" | "email-reader" | "doc-viewer" | "media-gallery"} ViewType
 */

/**
 * @typedef {"primary" | "secondary" | "danger" | "ghost"} ToolActionVariant
 */

/**
 * @typedef {object} ToolAction
 * @property {string} label
 * @property {string} intent
 * @property {ToolActionVariant=} variant
 */

/**
 * @typedef {object} ToolUi
 * @property {ViewType=} viewType
 * @property {string=} title
 * @property {string=} description
 * @property {ToolAction[]=} actions
 * @property {{source?: string, itemCount?: number, duration?: string, timestamp?: string, [key: string]: unknown}=} metadata
 */

/**
 * @typedef {object} ToolResponse
 * @property {boolean=} success
 * @property {any} data
 * @property {ToolUi=} ui
 * @property {string=} error
 */

export {};
