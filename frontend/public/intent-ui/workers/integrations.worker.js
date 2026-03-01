import { createIntegrationCatalog } from "../lib/integrations/catalog";

const catalog = createIntegrationCatalog();

function resolveIntegration(integrationId) {
  const id = String(integrationId || "").trim();
  return catalog[id] || null;
}

function reply(ok, id, result, error = "") {
  postMessage({
    ok,
    id,
    result: ok ? result : undefined,
    error: ok ? "" : String(error || "unknown worker error")
  });
}

self.onmessage = async (event) => {
  const data = event?.data || {};
  const requestId = String(data.id || "").trim();
  const type = String(data.type || "").trim();
  const payload = data.payload || {};
  const integration = resolveIntegration(payload.integrationId);

  if (!requestId) return;
  if (!integration) {
    reply(false, requestId, null, `unknown integration: ${payload.integrationId || ""}`);
    return;
  }

  try {
    if (type === "hydrate_connection") {
      const result = integration.hydrateConnection(payload);
      reply(true, requestId, result);
      return;
    }
    if (type === "connect_connection") {
      const result = integration.connectConnection(payload);
      reply(true, requestId, result);
      return;
    }
    if (type === "disconnect_connection") {
      const result = integration.disconnectConnection();
      reply(true, requestId, result);
      return;
    }
    if (type === "set_mode_connection") {
      const result = integration.setConnectorModeConnection(payload);
      reply(true, requestId, result);
      return;
    }
    if (type === "verify_integration") {
      const result = await integration.verifyConnection({ ...payload, fetchImpl: fetch });
      reply(true, requestId, result);
      return;
    }
    if (type === "list_connection_view") {
      const result = integration.listConnectionView(payload);
      reply(true, requestId, result);
      return;
    }
    reply(false, requestId, null, `unknown request type: ${type}`);
  } catch (error) {
    reply(false, requestId, null, error instanceof Error ? error.message : "integration worker execution failed");
  }
};
