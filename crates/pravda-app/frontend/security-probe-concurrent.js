"use strict";
// DEBUG SECURITY E2E ONLY. Runs in a capability-free WebView alongside main.
// Reports *only* rejection status over the test-only document title bridge.
const guarded = [
  "ipc_auth_status",
  "ipc_session_check",
  "ipc_market_snapshot",
  "ipc_history_recent",
  "ipc_history_query",
  "ipc_history_export_csv",
  "ipc_paper_alerts_snapshot",
  "ipc_runtime_metrics",
  "ipc_security_report",
  "ipc_security_events",
  "ipc_revolut_consent_get",
  "ipc_revolut_consent_set",
  "ipc_revolut_private_setup",
  "ipc_revolut_private_status",
  "ipc_revolut_private_consent_set",
  "ipc_revolut_private_balances",
  "ipc_fx_status",
  "plugin:fs|write_text_file",
  "plugin:shell|execute"
];
let sequence = 0;
async function check() {
  const invoke = window.__TAURI__?.core?.invoke;
  const results = [];
  const validArguments = {
    ipc_history_query: {
      query: {symbol:"BTC/USDT",exchange:"Binance",period:"day"}
    },
    ipc_revolut_consent_set: {enabled: false},
    ipc_revolut_private_consent_set: {enabled: false},
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
        /\bFORBIDDEN_WINDOW\b/i.test(description) ? "rust_window_guard" :
        /\bUNAUTHORIZED\b/i.test(description) ? "rust_auth" :
        /not allowed|permission denied|capability|access denied/i.test(description) ?
          "tauri_acl" : "other";
      results.push({command, state:"rejected",
        access_denied:denial_layer==="tauri_acl", denial_layer, malformed});
    }
  }
  // Compact wire keys: the Rust title callback drops any payload longer than
  // its bound (main.rs, `payload.len() > 3_000`), and a dropped report means
  // every deny verdict loses its evidence. The native E2E expands these keys
  // back to the explicit names its verdicts read (expandProbeResult).
  // Guarded by tools/tests/revolut-probe-channel.test.mjs, which executes this
  // real script and measures the emitted title against the compiled bound.
  const payload = JSON.stringify({v:1,seq:sequence++,results:results.map(row=>({
    c:row.command, s:row.state, a:row.access_denied,
    d:row.denial_layer, m:row.malformed
  }))});
  document.title = "PRAVDA_E2E_PROBE:" + btoa(payload);
}
void check();
setInterval(() => { void check(); }, 1500);
