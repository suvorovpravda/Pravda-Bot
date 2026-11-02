"use strict";
// DEBUG SECURITY E2E ONLY. Runs in a capability-free WebView alongside main.
// Reports *only* rejection status over the test-only document title bridge.
const guarded = [
  "ipc_auth_status",
  "ipc_market_snapshot",
  "ipc_history_export_csv"
];
let sequence = 0;
async function check() {
  const invoke = window.__TAURI__?.core?.invoke;
  const results = [];
  for (const command of guarded) {
    if (typeof invoke !== "function") {
      results.push({command, state:"no-bridge"});
      continue;
    }
    try {
      await invoke(command);
      results.push({command, state:"resolved"});
    } catch {
      results.push({command, state:"rejected"});
    }
  }
  const payload = JSON.stringify({v:1,seq:sequence++,results});
  document.title = "PRAVDA_E2E_PROBE:" + btoa(payload);
}
void check();
setInterval(() => { void check(); }, 1500);
