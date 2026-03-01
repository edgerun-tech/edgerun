const UI_INTENT_TOPICS = {
  window: {
    open: "intent.ui.window.open",
    close: "intent.ui.window.close",
    closeTop: "intent.ui.window.close_top",
    closeAll: "intent.ui.window.close_all",
    minimize: "intent.ui.window.minimize",
    maximize: "intent.ui.window.maximize",
    restore: "intent.ui.window.restore",
    focus: "intent.ui.window.focus",
    move: "intent.ui.window.move",
    resize: "intent.ui.window.resize"
  },
  clipboard: {
    push: "intent.ui.clipboard.push",
    clear: "intent.ui.clipboard.clear"
  },
  action: {
    intentBarToggle: "intent.ui.action.intentbar.toggle",
    profileBootstrapOpen: "intent.ui.action.profile_bootstrap.open",
    browserNavigate: "intent.ui.action.browser.navigate",
    callRing: "intent.ui.action.call.ring",
    terminalInput: "intent.ui.action.terminal.input",
    widgetsResetPositions: "intent.ui.action.widgets.reset_positions"
  },
  integration: {
    checkAll: "intent.ui.integration.check_all",
    connect: "intent.ui.integration.connect",
    disconnect: "intent.ui.integration.disconnect",
    setConnectorMode: "intent.ui.integration.set_connector_mode",
    verifyStarted: "intent.ui.integration.verify.started",
    verifySucceeded: "intent.ui.integration.verify.succeeded",
    verifyFailed: "intent.ui.integration.verify.failed"
  }
};

const UI_EVENT_TOPICS = {
  window: {
    opened: "ui.window.opened",
    closed: "ui.window.closed",
    minimized: "ui.window.minimized",
    maximized: "ui.window.maximized",
    restored: "ui.window.restored",
    focused: "ui.window.focused",
    moved: "ui.window.moved",
    resized: "ui.window.resized",
    closedAll: "ui.window.closed_all"
  },
  clipboard: {
    updated: "ui.clipboard.updated",
    cleared: "ui.clipboard.cleared"
  },
  action: {
    intentBarToggled: "ui.action.intentbar.toggled",
    profileBootstrapOpened: "ui.action.profile_bootstrap.opened",
    browserNavigated: "ui.action.browser.navigated",
    callRinging: "ui.action.call.ringing",
    terminalInputSent: "ui.action.terminal.input_sent",
    widgetsPositionsReset: "ui.action.widgets.positions_reset"
  },
  integration: {
    stateChanged: "ui.integration.state_changed",
    lifecycleChanged: "ui.integration.lifecycle.changed",
    verifyStarted: "ui.integration.verify_started",
    verified: "ui.integration.verified",
    verifyFailed: "ui.integration.verify_failed",
    flipperProbed: "ui.integration.flipper.probed",
    dalyBmsProbed: "ui.integration.daly_bms.probed"
  }
};

function uiIntentMeta(source, extra = {}) {
  return {
    source: String(source || "intent-ui"),
    domain: "ui",
    ...extra
  };
}

export {
  UI_EVENT_TOPICS,
  UI_INTENT_TOPICS,
  uiIntentMeta
};
