const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const test = require('node:test');

const html = fs.readFileSync(path.join(__dirname, '../dashboard.html'), 'utf8');
const start = html.indexOf('async function refreshNews()');
const end = html.indexOf('async function refreshCrypto()', start);
assert.ok(start >= 0 && end > start, 'The actual news renderer must be present');

class Element {
  constructor() {this.children = []; this.textContent = '';}
  append(...children) {this.children.push(...children);}
  replaceChildren(...children) {this.children = children;}
  set innerHTML(_) {throw new Error('Unsafe HTML insertion');}
}

test('The actual news renderer rejects script/data URLs and keeps titles as text', async () => {
  const nodes = new Map();
  const node = id => {
    if (!nodes.has(id)) nodes.set(id, new Element());
    return nodes.get(id);
  };
  const context = vm.createContext({
    URL, $: node, document: {createElement: () => new Element()},
    t: key => key, setStatus() {}, formatDate: String,
    fetchJson: async () => ({
      source: 'BBC World', retrieved_at: '2026-10-26T00:00:00Z',
      articles: [
        {url: 'javascript:alert(1)', title: 'Unsafe script'},
        {url: 'data:text/html,test', title: 'Unsafe document'},
        {url: 'not a URL', title: 'Malformed'},
        {url: 'https://example.org/news', title: '<img src=x onerror=alert(1)>'},
        {url: 'http://example.org/news', title: 'Reported event'},
      ],
    }),
  });
  vm.runInContext(html.slice(start, end), context, {filename: 'dashboard-news.js'});
  await context.refreshNews();
  const links = node('news-list').children.map(item => item.children[0]);
  assert.deepEqual(links.map(link => link.href), ['https://example.org/news', 'http://example.org/news']);
  assert.equal(links[0].textContent, '<img src=x onerror=alert(1)>');
  assert.ok(links.every(link => link.rel === 'noopener noreferrer' && link.target === '_blank'));
});
