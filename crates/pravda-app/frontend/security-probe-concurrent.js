"use strict";
// DEBUG SECURITY E2E ONLY. Runs in a capability-free WebView alongside main.
// Reports *only* rejection status over the test-only document title bridge.
const guarded = [
  "ipc_auth_status",
  "ipc_market_snapshot",
  "ipc_history_recent",
  "ipc_history_export_csv",
  "plugin:fs|write_text_file",
  "plugin:shell|execute"
];
let sequence = 0;
async function check() {
  const invoke = window.__TAURI__?.core?.invoke;
  const results = [];
  const validArguments = {
    ipc_history_export_csv: {
      query: {symbol:"BTC/USDT",exchange:"Binance",period:"day"}
    },
    "plugin:fs|write_text_file": {path:"__PRAVDA_E2E_NEVER_CREATE__",contents:"denied"},
    "plugin:shell|execute": {program:"__PRAVDA_E2E_FORBIDDEN_PROGRAM__"}
  };
  for (const command of guarded) {
    if (typeof invoke !== "function") {
      results.push({command, state:"no-bridge"});
      continue;
    }
    try {
      await invoke(command, validArguments[command] || {});
      results.push({command, state:"resolved"});
    } catch (error) {
      const description = String(error);
      const malformed = /invalid args|missing field|missing required|deserialize|invalid type|expected.*argument/i.test(description);
      // A missing command, malformed payload, or Rust UNAUTHORIZED is NOT
      // evidence that Tauri rejected the call before dispatching the handler.
      const denial_layer = malformed ? "malformed" :
        /\bUNAUTHORIZED\b/i.test(description) ? "rust_auth" :
        /not allowed|permission denied|capability|forbidden|access denied/i.test(description) ?
          "tauri_acl" : "other";
      results.push({command, state:"rejected",
        access_denied:denial_layer==="tauri_acl", denial_layer, malformed});
    }
  }
  const payload = JSON.stringify({v:1,seq:sequence++,results});
  document.title = "PRAVDA_E2E_PROBE:" + btoa(payload);
}
void check();
setInterval(() => { void check(); }, 1500);
