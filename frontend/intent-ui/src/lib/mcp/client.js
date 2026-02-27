const connected = /* @__PURE__ */ new Map();
const mcpManager = {
  async connectServer(server) {
    connected.set(server.id, server);
    return true;
  },
  getConnectedServers() {
    return Array.from(connected.values());
  }
};
export {
  mcpManager
};
