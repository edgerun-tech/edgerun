/**
 * Core types for the code graph viewer.
 */

/**
 * @typedef {Object} GraphNode
 * @property {string} id
 * @property {string} name
 * @property {string} [file]
 * @property {string} [language]
 * @property {string} [type]
 * @property {boolean} [is_static]
 * @property {number} [connections]
 * @property {string} [commit]
 * @property {number} [x]
 * @property {number} [y]
 * @property {number} [vx]
 * @property {number} [vy]
 * @property {boolean} [_hidden]
 * @property {number} [_renderIndex]
 */

/**
 * @typedef {Object} GraphEdge
 * @property {string} source
 * @property {string} target
 * @property {string} [kind]
 */

/**
 * @typedef {Object} GraphData
 * @property {GraphNode[]} nodes
 * @property {GraphEdge[]} edges
 */

/**
 * @typedef {Object} DiagnosticsItem
 * @property {string} file
 * @property {number} line
 * @property {number} column
 * @property {"error"|"warning"|"info"} severity
 * @property {string} message
 * @property {string} [code]
 */

/**
 * @typedef {Object} ChatMessage
 * @property {string} id
 * @property {"user"|"assistant"|"system"|"error"} role
 * @property {string} text
 * @property {ToolCall[]} [tools]
 * @property {number} timestamp
 */

/**
 * @typedef {Object} ToolCall
 * @property {string} name
 * @property {string} args
 * @property {string} [output]
 * @property {string} [content]
 */

/**
 * @typedef {Object} FileEntry
 * @property {string} name
 * @property {string} path
 * @property {"file"|"dir"} type
 */

/**
 * @typedef {Object} FileDependency
 * @property {string} file
 * @property {string[]} functions
 * @property {string[]} dependencies
 */

/**
 * @typedef {Object} RepoInfo
 * @property {string} path
 * @property {string} name
 * @property {boolean} is_git_repo
 * @property {number} [file_count]
 * @property {string[]} [languages]
 */

/**
 * @typedef {"sidebar"|"code"|"chat"|"diagnostics"|"file-explorer"|"repo"|null} PanelType
 */

/**
 * @typedef {"functions"|"files"|"directories"} ViewType
 */

// Benchmark comment