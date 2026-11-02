// Dependency-free regression checks for the inline dashboard script.
// Run: node tools/test-dashboard.mjs
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";

const html = readFileSync(new URL("../dashboard.html", import.meta.url), "utf8");
const scriptTag = html.match(/<script>([\s\S]*?)<\/script>/i);
assert.ok(scriptTag, "dashboard must contain its inline script");
const script = scriptTag[1];
new vm.Script(script, { filename: "dashboard.html" });

function between(start, end) {
    const a = script.indexOf(start);
    const b = script.indexOf(end, a + start.length);
    assert.ok(a >= 0 && b > a, "missing dashboard section " + start);
    return script.slice(a, b);
}

// EUR and CHF may not be available: quote-native USDT must still display.
const quoteHelpers = between("    function convertQuote(", "    const parsePrice =");
const prices = vm.runInNewContext([
    "let latestFx = null;",
    "const numericFormat = (v) => Number(v).toFixed(2);",
    "const formatCurrency = (v, c) => c + ':' + Number(v).toFixed(2);",
    quoteHelpers,
    "const withoutFx = { direct: convertQuote(120.25, 'USDT'), eur: convertQuote(120.25, 'EUR'), chf: convertQuote(120.25, 'CHF'), label: formatMarketValue(120.25, 'USDT') };",
    "latestFx = { rates: { USD: 1.2, CHF: 0.94 } };",
    "({ withoutFx, withFx: { eur: convertQuote(120, 'EUR'), chf: convertQuote(120, 'CHF') }, invalid: convertQuote(-1, 'USDT') });"
].join("\n"));
assert.equal(prices.withoutFx.direct, 120.25);
assert.equal(prices.withoutFx.eur, null);
assert.equal(prices.withoutFx.chf, null);
assert.equal(prices.withoutFx.label, "120.25 USDT");
assert.ok(Math.abs(prices.withFx.eur - 100) < 1e-9);
assert.ok(Math.abs(prices.withFx.chf - 94) < 1e-9);
assert.equal(prices.invalid, null);
assert.match(html, /id="chart-currency"><option>USDT<\/option>/);

// Paper alerts lack price, currency, ID and direction; they must still render.
// They must not expose a bogus Delete button targeting "undefined".
class MockElement {
    constructor() { this.children = []; this.textContent = ""; this.className = ""; }
    replaceChildren(...children) { this.children = children; this.textContent = ""; }
    append(...children) { this.children.push(...children); }
    addEventListener() {}
}
const alertRoot = new MockElement();
const context = {
    document: { createElement: () => new MockElement() },
    $: (id) => {
        assert.equal(id, "price-alerts");
        return alertRoot;
    },
    safeText: (value) => value == null ? "" : String(value),
    formatDate: (value) => String(value || "—"),
    formatCurrency: (value, currency) => currency + " " + value,
    t: (value) => value
};
const renderAlerts = between("    function renderAlerts(", "    async function refreshAlerts()");
vm.runInNewContext(renderAlerts + "\nrenderAlerts([{ symbol: 'BTC/USDT', message: 'Paper test event', timestamp: '2026-11-02T00:00:00Z', spread_percent: 0.5 }]);", context);
assert.equal(alertRoot.children.length, 1);
assert.equal(alertRoot.children[0].children.length, 1);
assert.match(alertRoot.children[0].children[0].textContent, /Paper test event/);
assert.doesNotMatch(alertRoot.children[0].children[0].textContent, /undefined|NaN/);

// The price form is explicitly unavailable until the Rust API can persist it.
assert.match(script, /price-alert-form"\)\.querySelectorAll\("input, select, button"\)/);
assert.match(script, /field\.disabled = true/);
for (const language of ["fr:", "ru:", "en:"]) {
    assert.ok(html.includes(language), "missing dashboard translation " + language);
}
assert.equal((script.match(/priceAlertsUnavailable:/g) || []).length, 3);

// Price snapshots are cheap to poll; history queries must not be sent each second.
const marketPoll = between("    async function refreshMarket()", "    async function refreshFx()");
assert.doesNotMatch(marketPoll, /refreshHistory\(/);
assert.match(script, /setInterval\(refreshMarket, 1000\)/);
assert.match(script, /setInterval\(refreshHistory, 30000\)/);
assert.doesNotMatch(script, /location\.reload\(\)/);
assert.match(script, /reconnectDelayMs = Math\.min\(reconnectDelayMs \* 2, 30000\)/);
assert.match(script, /formatDate\(quote\.timestamp\)/);

console.log("Dashboard syntax, USDT fallback, alert model and polling regression checks passed.");
