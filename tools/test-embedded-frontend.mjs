// PR-03 staged frontend validation; no browser or HTTP server required.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";

const root = new URL("../crates/pravda-app/frontend/", import.meta.url);
const original = readFileSync(new URL("../dashboard.html", import.meta.url), "utf8");
const html = readFileSync(new URL("index.html", root), "utf8");
const css = readFileSync(new URL("assets/styles.css", root), "utf8");
const js = readFileSync(new URL("assets/app.js", root), "utf8");
const conf = JSON.parse(readFileSync(new URL("../crates/pravda-app/tauri.conf.json", import.meta.url), "utf8"));

assert.equal(conf.build.frontendDist, "frontend", "frontend must be embedded from disk");
assert.equal(conf.bundle.active, false, "staging PR must not accidentally publish a bundle");
assert.deepEqual(conf.app.windows, [], "staging PR must keep the existing window lifecycle");
assert.match(html, /<link rel="stylesheet" href="\.\/assets\/styles\.css">/);
assert.match(html, /<script src="\.\/assets\/app\.js" defer><\/script>/);
assert.doesNotMatch(html, /<style(?:\s|>)/);
assert.doesNotMatch(html, /<script(?:\s|>)(?![^>]*src=)/);
assert.match(html, /Content-Security-Policy/);
assert.match(html, /script-src 'self'/);
assert.match(html, /connect-src 'self'/);
assert.doesNotMatch(html, /unsafe-inline|unsafe-eval/);
assert.doesNotMatch(html, /<script[^>]*src=["']https?:\/\//i);
assert.doesNotMatch(html, /<link[^>]*href=["']https?:\/\//i);
assert.doesNotMatch(css, /@import\s+url\(["']?https?:\/\//i);
new vm.Script(js, { filename: "crates/pravda-app/frontend/assets/app.js" });

const originalCss = original.match(/<style>([\s\S]*?)<\/style>/)?.[1].trim();
const originalJs = original.match(/<script>([\s\S]*?)<\/script>/)?.[1].trim();
assert.equal(css.trim(), originalCss, "embedded CSS must match legacy UI");
assert.equal(js.trim(), originalJs, "embedded JS must match legacy UI");
assert.match(html, /id="chart-currency"><option>USDT<\/option>/);

console.log("Bundled frontend assets, offline CSP, syntax and legacy parity passed.");
