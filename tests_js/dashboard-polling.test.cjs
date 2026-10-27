const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const test = require('node:test');

const html = fs.readFileSync(path.join(__dirname, '../dashboard.html'), 'utf8');

test('All inline scripts are closed, free of control bytes and valid JavaScript', () => {
  assert.doesNotMatch(html, /[\x00-\x08\x0b\x0c\x0e-\x1f]/);
  const scripts = [...html.matchAll(/<script\b[^>]*>([\s\S]*?)<\/script\s*>/gi)];
  assert.ok(scripts.length > 0);
  assert.equal(scripts.length, (html.match(/<script\b/gi) || []).length);
  for (const [index, script] of scripts.entries()) {
    assert.ok(script.index + script[0].length <= html.indexOf('</body>'));
    new vm.Script(script[1], {filename: `dashboard-inline-${index}.js`});
  }
});

test('Actual polling code registers each timer exactly once, including alerts', () => {
  const source = html.match(/    function startPolling\(\) \{([\s\S]*?)\n    \}/);
  assert.ok(source, 'The real polling function must exist');
  const registered = [];
  const periods = {
    updateClock: 1000, refreshMarket: 1000, refreshFx: 21600000,
    refreshFlows: 300000, refreshNews: 900000, refreshCrypto: 5000,
    refreshSources: 60000, checkPriceAlerts: 5000,
  };
  const functions = Object.fromEntries(Object.keys(periods).map(name => [name, () => {}]));
  vm.runInNewContext(source[0] + '\nstartPolling();', {
    ...functions, setInterval(callback, delay) {registered.push([callback, delay]);},
  });
  assert.equal(registered.length, Object.keys(periods).length);
  for (const [name, delay] of Object.entries(periods)) {
    assert.deepEqual(registered.filter(([callback]) => callback === functions[name]), [[functions[name], delay]]);
  }
  assert.equal((html.match(/^    startPolling\(\);$/gm) || []).length, 1);
});
