/**
 * @typedef {object} AppContext
 * @property {string} currentRepo
 * @property {string} currentBranch
 * @property {string} currentHost
 * @property {string} currentProject
 * @property {string[]} recentFiles
 * @property {string[]} recentCommands
 * @property {string[]} activeIntegrations
 * @property {string} environment
 * @property {string[]} openWindows
 */

/**
 * @typedef {object} ExecutionPlan
 * @property {string} id
 * @property {{raw: string, verb: string, target: string, modifiers: string[], context: unknown, confidence: number}} intent
 * @property {Array<{id: string, description: string, tool?: string}>} steps
 * @property {"low" | "high"} risk
 * @property {string[]} preview
 * @property {boolean=} requiresAuth
 * @property {string} predictedResult
 */

const intentProcessor = {
  /**
   * @param {string} query
   * @param {AppContext} appCtx
   * @returns {Promise<ExecutionPlan>}
   */
  async process(query, appCtx) {
    const lower = query.toLowerCase();
    const risk = lower.includes("delete") || lower.includes("drop") ? "high" : "low";
    return {
      id: `plan-${Date.now()}`,
      intent: {
        raw: query,
        verb: query.split(" ")[0] || "run",
        target: query.split(" ").slice(1).join(" ") || "task",
        modifiers: [],
        context: appCtx,
        confidence: 0.85
      },
      steps: [
        { id: "analyze", description: "Analyze request", tool: "intent-processor" },
        { id: "execute", description: "Execute with mock backend", tool: "intent-executor" }
      ],
      risk,
      preview: [`Will run: ${query}`],
      predictedResult: `Executed: ${query}`
    };
  }
};
export {
  intentProcessor
};
