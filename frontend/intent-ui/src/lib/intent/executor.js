/** @typedef {import("./processor").ExecutionPlan} ExecutionPlan */
/** @typedef {import("../mcp/types").ToolResponse} ToolResponse */

/**
 * @typedef {object} ExecuteResult
 * @property {boolean} success
 * @property {string} message
 * @property {ToolResponse[]=} responses
 */

const intentExecutor = {
  /**
   * @param {ExecutionPlan} plan
   * @returns {Promise<ExecuteResult>}
   */
  async execute(plan) {
    /** @type {ToolResponse} */
    const response = {
      success: true,
      data: {
        planId: plan.id,
        executedAt: (/* @__PURE__ */ new Date()).toISOString(),
        steps: plan.steps.length
      },
      ui: {
        title: "Execution Result",
        description: plan.predictedResult,
        viewType: "preview",
        metadata: {
          source: "Mock Executor",
          timestamp: (/* @__PURE__ */ new Date()).toISOString()
        }
      }
    };
    return {
      success: true,
      message: "Execution completed",
      responses: [response]
    };
  }
};
export {
  intentExecutor
};
